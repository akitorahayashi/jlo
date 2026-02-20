use chrono::Utc;
use serde::Serialize;

use crate::adapters::git::GitCommandAdapter;
use crate::adapters::github::GitHubCommandAdapter;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::AppError;
use crate::ports::{Git, GitHub, JulesStore};

const WORKER_PUSH_BRANCH_PREFIX: &str = "jules-worker-sync-";

#[derive(Debug, Clone)]
pub struct PushWorkerBranchOptions {
    pub change_token: String,
    pub commit_message: String,
    pub pr_title: String,
    pub pr_body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PushWorkerBranchOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
    pub merged: bool,
}

pub fn execute(options: PushWorkerBranchOptions) -> Result<PushWorkerBranchOutput, AppError> {
    let repository = LocalRepositoryAdapter::current()?;
    if !repository.jules_exists() {
        return Err(AppError::JulesNotFound);
    }

    let root = repository
        .jules_path()
        .parent()
        .ok_or_else(|| AppError::Validation("Invalid .jules path: missing parent".to_string()))?
        .to_path_buf();
    let git = GitCommandAdapter::new(root);
    let github = GitHubCommandAdapter::new();

    execute_with_adapters(&git, &github, options)
}

pub(crate) fn execute_with_adapters(
    git: &impl Git,
    github: &impl GitHub,
    options: PushWorkerBranchOptions,
) -> Result<PushWorkerBranchOutput, AppError> {
    validate_options(&options)?;

    let worker_branch = resolve_worker_branch_from_env()?;
    let current_branch = git.get_current_branch()?;
    if current_branch.trim() != worker_branch {
        return Err(AppError::Validation(format!(
            "workflow push worker-branch must run on '{}', current branch is '{}'",
            worker_branch, current_branch
        )));
    }

    git.fetch("origin")?;
    let has_local_commits = has_local_commits_ahead(git, &worker_branch)?;
    let status = git.run_command(&["status", "--porcelain", "--", ".jules"], None)?;
    let has_jules_changes = !status.trim().is_empty();
    if !has_local_commits && !has_jules_changes {
        return Ok(PushWorkerBranchOutput {
            schema_version: 1,
            applied: false,
            skipped_reason: Some("No local commits or .jules changes to push".to_string()),
            branch: None,
            pr_number: None,
            head_sha: None,
            merged: false,
        });
    }

    let push_branch = build_worker_push_branch_name(&options.change_token);
    git.checkout_branch(&push_branch, true)?;

    if has_jules_changes {
        git.run_command(&["add", "-A", "--", ".jules"], None)?;
        let staged = git.run_command(&["diff", "--cached", "--name-only"], None)?;
        if staged.trim().is_empty() && !has_local_commits {
            git.checkout_branch(&worker_branch, false)?;
            let _ = git.delete_branch(&push_branch, true)?;
            return Ok(PushWorkerBranchOutput {
                schema_version: 1,
                applied: false,
                skipped_reason: Some("No staged .jules changes to commit".to_string()),
                branch: None,
                pr_number: None,
                head_sha: None,
                merged: false,
            });
        }
        if !staged.trim().is_empty() {
            git.run_command(&["commit", "-m", &options.commit_message], None)?;
        }
    }

    let head_sha = git.get_head_sha()?;
    git.push_branch(&push_branch, false)?;

    let pr = match github.create_pull_request(
        &push_branch,
        &worker_branch,
        &options.pr_title,
        &options.pr_body,
    ) {
        Ok(pr) => pr,
        Err(err) => {
            let cleanup_error = github.delete_branch(&push_branch).err();
            return Err(with_cleanup_context(err, cleanup_error, None, &push_branch));
        }
    };

    // checks wait logic removed as requested checks

    if let Err(err) = github.merge_pull_request(pr.number) {
        let cleanup_error = cleanup_pr_and_branch(github, pr.number, &push_branch).err();
        return Err(with_cleanup_context(err, cleanup_error, Some(pr.number), &push_branch));
    }

    sync_worker_branch_to_origin(git, &worker_branch)?;

    Ok(PushWorkerBranchOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        branch: Some(push_branch),
        pr_number: Some(pr.number),
        head_sha: Some(head_sha),
        merged: true,
    })
}

fn has_local_commits_ahead(git: &impl Git, worker_branch: &str) -> Result<bool, AppError> {
    let remote_ref = format!("origin/{}", worker_branch);
    let range = format!("{}..HEAD", remote_ref);
    let output = git.run_command(&["rev-list", "--count", &range], None)?;
    let count = output.trim().parse::<u64>().map_err(|_| {
        AppError::Validation(format!(
            "Invalid commit count from 'git rev-list --count {}': {}",
            range,
            output.trim()
        ))
    })?;
    Ok(count > 0)
}

fn cleanup_pr_and_branch(
    github: &impl GitHub,
    pr_number: u64,
    push_branch: &str,
) -> Result<(), AppError> {
    github.close_pull_request(pr_number)?;
    github.delete_branch(push_branch)?;
    Ok(())
}

fn with_cleanup_context(
    cause: AppError,
    cleanup_error: Option<AppError>,
    pr_number: Option<u64>,
    push_branch: &str,
) -> AppError {
    if let Some(cleanup_error) = cleanup_error {
        let pr = pr_number.map(|number| number.to_string()).unwrap_or_else(|| "n/a".to_string());
        AppError::InternalError(format!(
            "worker-branch push failed: {} (cleanup failed for pr={}, branch='{}': {})",
            cause, pr, push_branch, cleanup_error
        ))
    } else {
        cause
    }
}

fn sync_worker_branch_to_origin(git: &impl Git, worker_branch: &str) -> Result<(), AppError> {
    // Worker-branch PRs are squash-merged, so local history can legitimately diverge.
    // Re-anchor the local worker branch to origin/<worker> explicitly.
    let remote_ref = format!("origin/{}", worker_branch);
    git.fetch("origin")?;
    git.run_command(&["checkout", "-B", worker_branch, remote_ref.as_str()], None)?;
    Ok(())
}

fn validate_options(options: &PushWorkerBranchOptions) -> Result<(), AppError> {
    if options.change_token.trim().is_empty() {
        return Err(AppError::Validation("change_token is required".to_string()));
    }
    if options.commit_message.trim().is_empty() {
        return Err(AppError::Validation("commit_message is required".to_string()));
    }
    if options.pr_title.trim().is_empty() {
        return Err(AppError::Validation("pr_title is required".to_string()));
    }
    if options.pr_body.trim().is_empty() {
        return Err(AppError::Validation("pr_body is required".to_string()));
    }
    Ok(())
}

fn resolve_worker_branch_from_env() -> Result<String, AppError> {
    let branch = std::env::var("JULES_WORKER_BRANCH").map_err(|_| {
        AppError::Validation(
            "JULES_WORKER_BRANCH environment variable is required for worker-branch push"
                .to_string(),
        )
    })?;
    if branch.trim().is_empty() {
        return Err(AppError::Validation(
            "JULES_WORKER_BRANCH environment variable must not be empty".to_string(),
        ));
    }
    Ok(branch.trim().to_string())
}

fn build_worker_push_branch_name(change_token: &str) -> String {
    let token = sanitize_branch_segment(change_token);
    let ts = Utc::now().format("%Y%m%d%H%M%S");
    format!("{}{}-{}", WORKER_PUSH_BRANCH_PREFIX, token, ts)
}

fn sanitize_branch_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
            out.push(ch);
        } else if ch.is_ascii_uppercase() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }

    let collapsed =
        out.split('-').filter(|segment| !segment.is_empty()).collect::<Vec<_>>().join("-");
    if collapsed.is_empty() { "change".to_string() } else { collapsed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::GitWorkspace;
    use crate::ports::{IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
    use serial_test::serial;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    struct EnvVarGuard {
        key: String,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set<K: Into<String>, V: AsRef<std::ffi::OsStr>>(key: K, value: V) -> Self {
            let key = key.into();
            let original = std::env::var_os(&key);
            // SAFETY: These tests are marked serial and never mutate env concurrently.
            unsafe {
                std::env::set_var(&key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(original) = self.original.as_ref() {
                // SAFETY: Guard is only used in serial tests.
                unsafe {
                    std::env::set_var(&self.key, original);
                }
            } else {
                // SAFETY: Guard is only used in serial tests.
                unsafe {
                    std::env::remove_var(&self.key);
                }
            }
        }
    }

    #[derive(Clone)]
    struct TestGit {
        current_branch: Arc<Mutex<String>>,
        status_output: String,
        staged_output: String,
        ahead_count: String,
        commands: Arc<Mutex<Vec<Vec<String>>>>,
        deleted_branches: Arc<Mutex<Vec<String>>>,
    }

    impl TestGit {
        fn new(
            current_branch: &str,
            status_output: &str,
            staged_output: &str,
            ahead_count: &str,
        ) -> Self {
            Self {
                current_branch: Arc::new(Mutex::new(current_branch.to_string())),
                status_output: status_output.to_string(),
                staged_output: staged_output.to_string(),
                ahead_count: ahead_count.to_string(),
                commands: Arc::new(Mutex::new(Vec::new())),
                deleted_branches: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl Git for TestGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            Ok("abc123head".to_string())
        }

        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok(self.current_branch.lock().expect("branch lock poisoned").clone())
        }

        fn commit_exists(&self, _sha: &str) -> bool {
            true
        }

        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<Option<String>, AppError> {
            Ok(Some("ancestor".to_string()))
        }

        fn get_first_commit(&self, _commit: &str) -> Result<String, AppError> {
            Ok("root".to_string())
        }

        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            Ok(false)
        }

        fn run_command(&self, args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
            self.commands
                .lock()
                .expect("commands lock poisoned")
                .push(args.iter().map(|arg| arg.to_string()).collect());

            if args == ["status", "--porcelain", "--", ".jules"] {
                return Ok(self.status_output.clone());
            }
            if args == ["diff", "--cached", "--name-only"] {
                return Ok(self.staged_output.clone());
            }
            if args.first().copied() == Some("rev-list") && args.get(1).copied() == Some("--count")
            {
                return Ok(self.ahead_count.clone());
            }
            Ok(String::new())
        }

        fn checkout_branch(&self, branch: &str, _create: bool) -> Result<(), AppError> {
            *self.current_branch.lock().expect("branch lock poisoned") = branch.to_string();
            Ok(())
        }

        fn push_branch(&self, _branch: &str, _force: bool) -> Result<(), AppError> {
            Ok(())
        }

        fn push_branch_from_rev(
            &self,
            _rev: &str,
            _branch: &str,
            _force: bool,
        ) -> Result<(), AppError> {
            Ok(())
        }

        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            Ok("commit-sha".to_string())
        }

        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn delete_branch(&self, branch: &str, _force: bool) -> Result<bool, AppError> {
            self.deleted_branches
                .lock()
                .expect("deleted branches lock poisoned")
                .push(branch.to_string());
            Ok(true)
        }

        fn create_workspace(&self, _branch: &str) -> Result<Box<dyn GitWorkspace>, AppError> {
            unimplemented!()
        }
    }

    #[derive(Clone)]
    struct TestGitHub {
        should_fail_create_pr: bool,
        should_fail_merge: bool,
        created_head: Arc<Mutex<Option<String>>>,
        closed_prs: Arc<Mutex<Vec<u64>>>,
        merged_prs: Arc<Mutex<Vec<u64>>>,
        deleted_remote_branches: Arc<Mutex<Vec<String>>>,
    }

    impl TestGitHub {
        fn new(should_fail_create_pr: bool, should_fail_merge: bool) -> Self {
            Self {
                should_fail_create_pr,
                should_fail_merge,
                created_head: Arc::new(Mutex::new(None)),
                closed_prs: Arc::new(Mutex::new(Vec::new())),
                merged_prs: Arc::new(Mutex::new(Vec::new())),
                deleted_remote_branches: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl GitHub for TestGitHub {
        fn create_pull_request(
            &self,
            head: &str,
            _base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<PullRequestInfo, AppError> {
            if self.should_fail_create_pr {
                return Err(AppError::ExternalToolError {
                    tool: "github".to_string(),
                    error: "create pr failed".to_string(),
                });
            }

            *self.created_head.lock().expect("created head lock poisoned") = Some(head.to_string());
            Ok(PullRequestInfo {
                number: 77,
                url: "https://example.test/pr/77".to_string(),
                head: head.to_string(),
                base: "jules".to_string(),
            })
        }

        fn close_pull_request(&self, pr_number: u64) -> Result<(), AppError> {
            self.closed_prs.lock().expect("closed prs lock poisoned").push(pr_number);
            Ok(())
        }

        fn delete_branch(&self, branch: &str) -> Result<(), AppError> {
            self.deleted_remote_branches
                .lock()
                .expect("deleted remote branches lock poisoned")
                .push(branch.to_string());
            Ok(())
        }

        fn create_issue(
            &self,
            _title: &str,
            _body: &str,
            _labels: &[&str],
        ) -> Result<IssueInfo, AppError> {
            Ok(IssueInfo { number: 1, url: "https://example.test/issues/1".to_string() })
        }

        fn get_pr_detail(&self, _pr_number: u64) -> Result<PullRequestDetail, AppError> {
            Ok(PullRequestDetail {
                number: 1,
                head: "head".to_string(),
                base: "base".to_string(),
                is_draft: false,
                auto_merge_enabled: false,
            })
        }

        fn list_pr_comments(&self, _pr_number: u64) -> Result<Vec<PrComment>, AppError> {
            Ok(Vec::new())
        }

        fn create_pr_comment(&self, _pr_number: u64, _body: &str) -> Result<u64, AppError> {
            Ok(1)
        }

        fn update_pr_comment(&self, _comment_id: u64, _body: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn ensure_label(&self, _label: &str, _color: Option<&str>) -> Result<(), AppError> {
            Ok(())
        }

        fn add_label_to_pr(&self, _pr_number: u64, _label: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn add_label_to_issue(&self, _issue_number: u64, _label: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn enable_automerge(&self, _pr_number: u64) -> Result<(), AppError> {
            Ok(())
        }

        fn list_pr_files(&self, _pr_number: u64) -> Result<Vec<String>, AppError> {
            Ok(Vec::new())
        }

        fn merge_pull_request(&self, pr_number: u64) -> Result<(), AppError> {
            if self.should_fail_merge {
                return Err(AppError::ExternalToolError {
                    tool: "github".to_string(),
                    error: "merge failed".to_string(),
                });
            }
            self.merged_prs.lock().expect("merged prs lock poisoned").push(pr_number);
            Ok(())
        }
    }

    fn options() -> PushWorkerBranchOptions {
        PushWorkerBranchOptions {
            change_token: "cleanup".to_string(),
            commit_message: "jules: cleanup".to_string(),
            pr_title: "chore: cleanup".to_string(),
            pr_body: "cleanup details".to_string(),
        }
    }

    #[test]
    fn sanitize_branch_segment_normalizes_value() {
        assert_eq!(sanitize_branch_segment("Mock Cleanup/Run #1"), "mock-cleanup-run-1");
    }

    #[test]
    fn branch_name_uses_expected_prefix() {
        let branch = build_worker_push_branch_name("requirement-cleanup");
        assert!(branch.starts_with(WORKER_PUSH_BRANCH_PREFIX));
        assert!(branch.contains("requirement-cleanup-"));
    }

    #[test]
    #[serial]
    fn execute_with_adapters_succeeds_and_resyncs_worker_branch_to_origin() {
        let _worker_branch = EnvVarGuard::set("JULES_WORKER_BRANCH", "jules");
        let git = TestGit::new(
            "jules",
            " M .jules/schemas/observers/event.yml",
            ".jules/schemas/observers/event.yml\n",
            "0",
        );
        let github = TestGitHub::new(false, false);

        let out = execute_with_adapters(&git, &github, options()).expect("push should succeed");

        assert!(out.applied);
        assert!(out.merged);
        assert_eq!(out.pr_number, Some(77));

        let commands = git.commands.lock().expect("commands lock poisoned");
        assert!(
            commands.iter().any(|cmd| cmd == &vec!["checkout", "-B", "jules", "origin/jules"]),
            "worker branch should be reset to origin after merge"
        );
    }

    #[test]
    #[serial]
    fn execute_with_adapters_deletes_local_push_branch_when_nothing_staged() {
        let _worker_branch = EnvVarGuard::set("JULES_WORKER_BRANCH", "jules");
        let git = TestGit::new("jules", " M .jules/schemas/observers/event.yml", "", "0");
        let github = TestGitHub::new(false, false);

        let out = execute_with_adapters(&git, &github, options()).expect("should skip cleanly");

        assert!(!out.applied);
        assert_eq!(out.skipped_reason.as_deref(), Some("No staged .jules changes to commit"));

        let deleted = git.deleted_branches.lock().expect("deleted branches lock poisoned");
        assert_eq!(deleted.len(), 1);
        assert!(deleted[0].starts_with(WORKER_PUSH_BRANCH_PREFIX));
    }

    #[test]
    #[serial]
    fn execute_with_adapters_attempts_cleanup_when_merge_fails() {
        let _worker_branch = EnvVarGuard::set("JULES_WORKER_BRANCH", "jules");
        let git = TestGit::new(
            "jules",
            " M .jules/schemas/observers/event.yml",
            ".jules/schemas/observers/event.yml\n",
            "0",
        );
        let github = TestGitHub::new(false, true); // fail merge

        let err = execute_with_adapters(&git, &github, options())
            .expect_err("merge failure should return error");

        assert!(matches!(err, AppError::ExternalToolError { .. }));

        let closed_prs = github.closed_prs.lock().expect("closed prs lock poisoned");
        assert_eq!(*closed_prs, vec![77]);

        let deleted_remote =
            github.deleted_remote_branches.lock().expect("deleted remote lock poisoned");
        assert_eq!(deleted_remote.len(), 1);
        assert!(deleted_remote[0].starts_with(WORKER_PUSH_BRANCH_PREFIX));
    }

    #[test]
    #[serial]
    fn execute_with_adapters_pushes_existing_local_commits_without_new_jules_commit() {
        let _worker_branch = EnvVarGuard::set("JULES_WORKER_BRANCH", "jules");
        let git = TestGit::new("jules", "", "", "2");
        let github = TestGitHub::new(false, false);

        let out = execute_with_adapters(&git, &github, options()).expect("push should succeed");
        assert!(out.applied);
        assert!(out.merged);

        let commands = git.commands.lock().expect("commands lock poisoned");
        assert!(
            !commands.iter().any(|cmd| cmd == &vec!["add", "-A", "--", ".jules"]),
            "command should not stage .jules when there are no working-tree .jules changes"
        );
        assert!(
            !commands.iter().any(|cmd| cmd == &vec!["commit", "-m", "jules: cleanup"]),
            "command should not create extra commit when only local commits need publishing"
        );
    }

    #[test]
    #[serial]
    fn execute_with_adapters_skips_when_no_local_or_jules_changes() {
        let _worker_branch = EnvVarGuard::set("JULES_WORKER_BRANCH", "jules");
        let git = TestGit::new("jules", "", "", "0");
        let github = TestGitHub::new(false, false);

        let out = execute_with_adapters(&git, &github, options()).expect("should skip cleanly");
        assert!(!out.applied);
        assert_eq!(out.skipped_reason.as_deref(), Some("No local commits or .jules changes to push"));
    }
}
