//! Workflow bootstrap worker-branch subcommand.
//!
//! Ensures the worker branch exists and synchronizes target branch changes into it.

use serde::Serialize;

use crate::adapters::git::GitCommandAdapter;
use crate::domain::AppError;
use crate::ports::Git;

/// Options for `workflow bootstrap worker-branch`.
#[derive(Debug)]
pub struct WorkflowBootstrapWorkerBranchOptions {
    /// Root path of the repository.
    pub root: std::path::PathBuf,
}

/// Output of `workflow bootstrap worker-branch`.
#[derive(Debug, Serialize)]
pub struct WorkflowBootstrapWorkerBranchOutput {
    pub schema_version: u32,
    pub target_branch: String,
    pub worker_branch: String,
    pub worker_created: bool,
    pub merged: bool,
    pub conflict_resolved: bool,
}

/// Execute `workflow bootstrap worker-branch`.
pub fn execute(
    options: WorkflowBootstrapWorkerBranchOptions,
) -> Result<WorkflowBootstrapWorkerBranchOutput, AppError> {
    let git = GitCommandAdapter::new(options.root);
    let target_branch = read_required_branch_env("JLO_TARGET_BRANCH")?;
    let worker_branch = read_required_branch_env("JULES_WORKER_BRANCH")?;

    execute_with_adapter(&git, target_branch.as_str(), worker_branch.as_str())
}

pub(crate) fn execute_with_adapter(
    git: &impl Git,
    target_branch: &str,
    worker_branch: &str,
) -> Result<WorkflowBootstrapWorkerBranchOutput, AppError> {
    validate_branch_name(target_branch, "JLO_TARGET_BRANCH")?;
    validate_branch_name(worker_branch, "JULES_WORKER_BRANCH")?;
    if target_branch == worker_branch {
        return Err(AppError::Validation(
            "JLO_TARGET_BRANCH and JULES_WORKER_BRANCH must be different".to_string(),
        ));
    }

    git.run_command(&["fetch", "origin", target_branch], None)?;

    let worker_exists = remote_branch_exists(git, worker_branch)?;
    let target_ref = format!("origin/{}", target_branch);

    let (workspace_base, worker_created) = if worker_exists {
        git.run_command(&["fetch", "origin", worker_branch], None)?;
        (format!("origin/{}", worker_branch), false)
    } else {
        git.push_branch_from_rev(target_ref.as_str(), worker_branch, false)?;
        (target_ref.clone(), true)
    };

    let workspace = git.create_workspace(&workspace_base)?;

    let mut conflict_resolved = false;
    let merge_result = workspace.run_command(
        &["merge", target_ref.as_str(), "--no-edit", "-X", "theirs", "--allow-unrelated-histories"],
        None,
    );

    if merge_result.is_err() {
        workspace.run_command(&["checkout", "--ours", ".jules/"], None)?;
        workspace.run_command(&["add", ".jules/"], None)?;
        let staged = workspace.run_command(&["diff", "--cached", "--name-only"], None)?;
        if staged.trim().is_empty() {
            return Err(AppError::Validation(
                "Merge failed and no .jules conflict-resolution changes were staged".to_string(),
            ));
        }
        workspace.run_command(&["commit", "--no-edit"], None)?;
        conflict_resolved = true;
    }

    workspace.push_branch_from_rev("HEAD", worker_branch, false)?;

    Ok(WorkflowBootstrapWorkerBranchOutput {
        schema_version: 1,
        target_branch: target_branch.to_string(),
        worker_branch: worker_branch.to_string(),
        worker_created,
        merged: true,
        conflict_resolved,
    })
}

fn remote_branch_exists(git: &impl Git, branch: &str) -> Result<bool, AppError> {
    let out = git.run_command(&["ls-remote", "--heads", "origin", branch], None)?;
    Ok(!out.trim().is_empty())
}

fn read_required_branch_env(key: &str) -> Result<String, AppError> {
    std::env::var(key).map_err(|_| AppError::EnvironmentVariableMissing(key.to_string()))
}

fn validate_branch_name(value: &str, key: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{} must not be empty", key)));
    }
    if value.starts_with('-') {
        return Err(AppError::Validation(format!("{} must not start with '-'", key)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::GitWorkspace;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct TestGit {
        ls_remote_output: String,
        merge_fails: bool,
        staged_after_conflict: String,
        commands: Arc<Mutex<Vec<Vec<String>>>>,
        pushed: Arc<Mutex<Vec<String>>>,
    }

    impl TestGit {
        fn new(ls_remote_output: &str, merge_fails: bool, staged_after_conflict: &str) -> Self {
            Self {
                ls_remote_output: ls_remote_output.to_string(),
                merge_fails,
                staged_after_conflict: staged_after_conflict.to_string(),
                commands: Arc::new(Mutex::new(Vec::new())),
                pushed: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl Git for TestGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            Ok("head".to_string())
        }

        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok("main".to_string())
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
                .push(args.iter().map(|a| a.to_string()).collect());

            if args.starts_with(&["ls-remote", "--heads", "origin"]) {
                return Ok(self.ls_remote_output.clone());
            }
            if args.first().copied() == Some("merge") && self.merge_fails {
                return Err(AppError::GitError {
                    command: "git merge".to_string(),
                    details: "merge conflict".to_string(),
                });
            }
            if args == ["diff", "--cached", "--name-only"] {
                return Ok(self.staged_after_conflict.clone());
            }

            Ok(String::new())
        }

        fn checkout_branch(&self, _branch: &str, _create: bool) -> Result<(), AppError> {
            Ok(())
        }

        fn push_branch(&self, branch: &str, _force: bool) -> Result<(), AppError> {
            self.pushed.lock().expect("pushed lock poisoned").push(branch.to_string());
            Ok(())
        }

        fn push_branch_from_rev(&self, _rev: &str, branch: &str, _force: bool) -> Result<(), AppError> {
            self.pushed.lock().expect("pushed lock poisoned").push(branch.to_string());
            Ok(())
        }

        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            Ok("head".to_string())
        }

        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(false)
        }

        fn create_workspace(&self, _branch: &str) -> Result<Box<dyn GitWorkspace>, AppError> {
            Ok(Box::new(TestGitWorkspace { git: self.clone() }))
        }
    }

    struct TestGitWorkspace {
        git: TestGit,
    }

    impl Git for TestGitWorkspace {
        fn get_head_sha(&self) -> Result<String, AppError> {
            self.git.get_head_sha()
        }
        fn get_current_branch(&self) -> Result<String, AppError> {
            self.git.get_current_branch()
        }
        fn commit_exists(&self, sha: &str) -> bool {
            self.git.commit_exists(sha)
        }
        fn get_nth_ancestor(&self, commit: &str, n: usize) -> Result<Option<String>, AppError> {
            self.git.get_nth_ancestor(commit, n)
        }
        fn get_first_commit(&self, commit: &str) -> Result<String, AppError> {
            self.git.get_first_commit(commit)
        }
        fn has_changes(
            &self,
            from: &str,
            to: &str,
            pathspec: &[&str],
        ) -> Result<bool, AppError> {
            self.git.has_changes(from, to, pathspec)
        }
        fn run_command(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, AppError> {
            self.git.run_command(args, cwd)
        }
        fn checkout_branch(&self, branch: &str, create: bool) -> Result<(), AppError> {
            self.git.checkout_branch(branch, create)
        }
        fn push_branch(&self, branch: &str, force: bool) -> Result<(), AppError> {
            self.git.push_branch(branch, force)
        }
        fn push_branch_from_rev(&self, rev: &str, branch: &str, force: bool) -> Result<(), AppError> {
            self.git.push_branch_from_rev(rev, branch, force)
        }
        fn commit_files(&self, message: &str, files: &[&Path]) -> Result<String, AppError> {
            self.git.commit_files(message, files)
        }
        fn fetch(&self, remote: &str) -> Result<(), AppError> {
            self.git.fetch(remote)
        }
        fn delete_branch(&self, branch: &str, force: bool) -> Result<bool, AppError> {
            self.git.delete_branch(branch, force)
        }
        fn create_workspace(&self, branch: &str) -> Result<Box<dyn GitWorkspace>, AppError> {
            self.git.create_workspace(branch)
        }
    }

    impl GitWorkspace for TestGitWorkspace {
        fn path(&self) -> &Path {
            Path::new("/tmp/test-workspace")
        }
    }

    #[test]
    fn creates_worker_branch_when_missing() {
        let git = TestGit::new("", false, "");
        let out = execute_with_adapter(&git, "main", "jules").expect("worker branch sync failed");
        assert!(out.worker_created);
        assert!(out.merged);
        assert!(!out.conflict_resolved);
        // Pushed once when created, once after merge
        assert_eq!(git.pushed.lock().expect("pushed lock poisoned").as_slice(), ["jules", "jules"]);
    }

    #[test]
    fn reuses_existing_worker_branch_when_present() {
        let git = TestGit::new("sha\trefs/heads/jules", false, "");
        let out = execute_with_adapter(&git, "main", "jules").expect("worker branch sync failed");
        assert!(!out.worker_created);
        // Pushed once after merge
        assert_eq!(git.pushed.lock().expect("pushed lock poisoned").as_slice(), ["jules"]);
    }

    #[test]
    fn resolves_merge_conflict_with_jules_policy() {
        let git =
            TestGit::new("sha\trefs/heads/jules", true, ".jules/schemas/narrator/changes.yml");
        let out = execute_with_adapter(&git, "main", "jules").expect("worker branch sync failed");
        assert!(out.conflict_resolved);

        let commands = git.commands.lock().expect("commands lock poisoned");
        assert!(commands.iter().any(|args| args == &vec!["checkout", "--ours", ".jules/"]));
        assert!(commands.iter().any(|args| args == &vec!["add", ".jules/"]));
    }

    #[test]
    fn fails_when_conflict_resolution_stages_nothing() {
        let git = TestGit::new("sha\trefs/heads/jules", true, "");
        let err =
            execute_with_adapter(&git, "main", "jules").expect_err("expected conflict failure");
        assert!(err.to_string().contains("no .jules conflict-resolution changes were staged"));
    }
}
