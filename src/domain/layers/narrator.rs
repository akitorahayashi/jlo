use std::path::Path;

use chrono::DateTime;

use crate::domain::configuration::loader::detect_repository_source;
use crate::domain::configuration::mock_loader::load_mock_config;
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, MockConfig, MockOutput, RunConfig, RunOptions};
use crate::ports::{AutomationMode, GitHubPort, GitPort, SessionRequest, WorkspaceStore};

use super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct NarratorLayer;

impl<W> LayerStrategy<W> for NarratorLayer
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn GitPort,
        _github: &dyn GitHubPort,
        workspace: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if options.mock {
            let mock_config = load_mock_config(jules_path, options, workspace)?;
            let output = execute_mock(&mock_config)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::mock_utils::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::mock_utils::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec!["narrator".to_string()],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            options.prompt_preview,
            options.branch.as_deref(),
            config,
            git,
            workspace,
            client_factory,
        )
    }
}

/// Execute the Narrator layer in real mode.
fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    config: &RunConfig,
    git: &G,
    workspace: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
    G: GitPort + ?Sized,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let changes_path = exchange_changes_path(jules_path)?;
    let had_previous_changes = workspace.file_exists(&changes_path);

    // Determine starting branch (Narrator always uses jules worker branch)
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_worker_branch.clone());

    // Determine commit range
    let range = determine_range(&changes_path, git, workspace)?;

    // Check if there are any non-excluded changes in the range
    let pathspec = &[".", ":(exclude).jules"];
    let has_changes = git.has_changes(&range.from_commit, &range.to_commit, pathspec)?;

    if !has_changes {
        println!("No codebase changes detected (excluding .jules/). Skipping Narrator.");
        return Ok(RunResult {
            roles: vec!["narrator".to_string()],
            prompt_preview,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let prompt = assemble_narrator_prompt(jules_path, &range, git, workspace)?;

    if prompt_preview {
        println!("=== Prompt Preview: Narrator ===");
        println!("Starting branch: {}\n", starting_branch);
        println!("{}", prompt);
        return Ok(RunResult {
            roles: vec!["narrator".to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    if had_previous_changes {
        workspace.remove_file(&changes_path)?;
        println!("Removed previous .jules/exchange/changes.yml after reading created_at cursor.");
    }

    // Create session
    let source = detect_repository_source(git)?;
    let client = client_factory.create()?;

    let request = SessionRequest {
        prompt,
        source,
        starting_branch,
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    match client.create_session(request) {
        Ok(response) => {
            println!("✅ Narrator session created: {}", response.session_id);
            Ok(RunResult {
                roles: vec!["narrator".to_string()],
                prompt_preview: false,
                sessions: vec![response.session_id],
                cleanup_requirement: None,
            })
        }
        Err(e) => {
            println!("❌ Failed to create Narrator session: {}", e);
            Err(e)
        }
    }
}

fn execute_mock(config: &MockConfig) -> Result<MockOutput, AppError> {
    let _ = config.branch_prefix(Layer::Narrator)?;
    println!("Mock narrator: no-op (preserving existing .jules/exchange/changes.yml)");

    Ok(MockOutput {
        mock_branch: String::new(),
        mock_pr_number: 0,
        mock_pr_url: String::new(),
        mock_tag: config.mock_tag.clone(),
    })
}

// --- Prompt Assembly Logic ---

fn assemble_narrator_prompt<
    G: GitPort + ?Sized,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
>(
    jules_path: &Path,
    range: &RangeContext,
    git: &G,
    workspace: &W,
) -> Result<String, AppError> {
    let run_mode = match range.selection_mode.as_str() {
        "incremental" => "overwrite",
        other => other,
    };

    let mut prompt_context = PromptContext::new()
        .with_var("run_mode", run_mode)
        .with_var("range_description", build_range_description(range));

    // Provide commit list since cursor (non-empty only for overwrite mode)
    let commits_text = if run_mode == "overwrite" {
        fetch_commits_since_cursor(git, range)?
    } else {
        String::new()
    };
    prompt_context = prompt_context.with_var("commits_since_cursor", commits_text);

    assemble_prompt(jules_path, Layer::Narrator, &prompt_context, workspace)
        .map(|p: AssembledPrompt| p.content)
        .map_err(|e| AppError::InternalError(e.to_string()))
}

fn fetch_commits_since_cursor<G: GitPort + ?Sized>(
    git: &G,
    range: &RangeContext,
) -> Result<String, AppError> {
    let since = range.changes_since.as_deref().unwrap_or("");
    if since.is_empty() {
        return Ok(String::new());
    }
    let after_arg = format!("--after={}", since);
    let output = git.run_command(
        &["log", &after_arg, "--format=%H %ai %s", "--", ".", ":(exclude).jules", ":(exclude).jlo"],
        None,
    )?;
    Ok(output.trim().to_string())
}

// --- Range Logic ---

/// Number of commits to use for bootstrap when no prior summary exists.
pub const BOOTSTRAP_COMMIT_COUNT: usize = 20;

#[derive(Debug, PartialEq)]
struct RangeContext {
    from_commit: String,
    to_commit: String,
    selection_mode: String,
    selection_detail: String,
    changes_since: Option<String>,
}

fn determine_range<G, W>(
    changes_path: &str,
    git: &G,
    workspace: &W,
) -> Result<RangeContext, AppError>
where
    G: GitPort + ?Sized,
    W: WorkspaceStore,
{
    let head_sha = git.get_head_sha()?;
    let changes_content = if workspace.file_exists(changes_path) {
        Some(workspace.read_file(changes_path)?)
    } else {
        None
    };

    determine_range_strategy(
        &head_sha,
        changes_content.as_deref(),
        |sha, n| git.get_nth_ancestor(sha, n),
        |sha, timestamp| {
            let commit = get_commit_before_timestamp(git, sha, timestamp)?;
            if let Some(ref base) = commit
                && !git.commit_exists(base)
            {
                return Err(AppError::Validation(format!(
                    "Resolved base commit does not exist: {}",
                    base
                )));
            }
            Ok(commit)
        },
    )
}

fn determine_range_strategy(
    head_sha: &str,
    latest_yml_content: Option<&str>,
    get_bootstrap_commit: impl Fn(&str, usize) -> Result<String, AppError>,
    get_commit_before_time: impl Fn(&str, &str) -> Result<Option<String>, AppError>,
) -> Result<RangeContext, AppError> {
    if let Some(content) = latest_yml_content {
        let previous_created_at = extract_created_at(content)?;
        if let Some(base_commit) = get_commit_before_time(head_sha, &previous_created_at)? {
            return Ok(RangeContext {
                from_commit: base_commit,
                to_commit: head_sha.to_string(),
                selection_mode: "incremental".to_string(),
                selection_detail: String::new(),
                changes_since: Some(previous_created_at),
            });
        }

        // Explicit fallback: no commit exists before the cursor time.
        let bootstrap_from = get_bootstrap_commit(head_sha, BOOTSTRAP_COMMIT_COUNT)?;
        return Ok(RangeContext {
            from_commit: bootstrap_from,
            to_commit: head_sha.to_string(),
            selection_mode: "bootstrap".to_string(),
            selection_detail: format!(
                "Fallback bootstrap: no commit found before previous created_at ({})",
                previous_created_at
            ),
            changes_since: Some(previous_created_at),
        });
    }

    // Bootstrap: use recent commits
    let bootstrap_from = get_bootstrap_commit(head_sha, BOOTSTRAP_COMMIT_COUNT)?;
    Ok(RangeContext {
        from_commit: bootstrap_from,
        to_commit: head_sha.to_string(),
        selection_mode: "bootstrap".to_string(),
        selection_detail: format!(
            "Last {} commits with non-.jules/ changes",
            BOOTSTRAP_COMMIT_COUNT
        ),
        changes_since: None,
    })
}

fn extract_created_at(content: &str) -> Result<String, AppError> {
    let data =
        serde_yaml::from_str::<serde_yaml::Value>(content).map_err(|err| AppError::ParseError {
            what: ".jules/exchange/changes.yml".to_string(),
            details: err.to_string(),
        })?;

    let created_at =
        data.get("created_at").and_then(|val| val.as_str()).filter(|s| !s.is_empty()).ok_or_else(
            || {
                AppError::Validation(
                    "changes.yml must contain non-empty created_at for incremental narrator runs"
                        .to_string(),
                )
            },
        )?;

    DateTime::parse_from_rfc3339(created_at).map_err(|err| {
        AppError::Validation(format!(
            "changes.yml created_at must be RFC3339: {} ({})",
            created_at, err
        ))
    })?;

    Ok(created_at.to_string())
}

fn get_commit_before_timestamp<G: GitPort + ?Sized>(
    git: &G,
    head_sha: &str,
    timestamp: &str,
) -> Result<Option<String>, AppError> {
    let before_arg = format!("--before={}", timestamp);
    let output = git.run_command(&["rev-list", "-1", &before_arg, head_sha], None)?;
    let commit = output.trim();
    if commit.is_empty() { Ok(None) } else { Ok(Some(commit.to_string())) }
}

fn build_range_description(range: &RangeContext) -> String {
    let short_from = &range.from_commit[..7.min(range.from_commit.len())];
    let short_to = &range.to_commit[..7.min(range.to_commit.len())];

    match range.selection_mode.as_str() {
        "incremental" => {
            let since = range.changes_since.as_deref().unwrap_or("unknown");
            format!("Summarize changes since {} (commits {}..{}).", since, short_from, short_to)
        }
        "bootstrap" => {
            let detail = if range.selection_detail.is_empty() {
                "recent commits".to_string()
            } else {
                range.selection_detail.clone()
            };
            format!("First summary — {} (commits {}..{}).", detail, short_from, short_to)
        }
        _ => {
            format!("Commits {}..{}.", short_from, short_to)
        }
    }
}

fn exchange_changes_path(jules_path: &Path) -> Result<String, AppError> {
    jules::exchange_changes(jules_path)
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Validation("Jules path contains invalid unicode".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{DiscoveredRole, PullRequestInfo, ScaffoldFile};
    use std::collections::HashMap;
    use std::path::PathBuf;

    // --- Range strategy tests ---

    #[test]
    fn test_extract_created_at_valid() {
        let content = r#"
created_at: "2026-02-05T00:00:00Z"
"#;
        let created_at = extract_created_at(content).unwrap();
        assert_eq!(created_at, "2026-02-05T00:00:00Z");
    }

    #[test]
    fn test_determine_range_strategy_incremental() {
        let head = "head_sha";
        let latest = r#"
created_at: "2026-02-05T00:00:00Z"
"#;
        let result = determine_range_strategy(
            head,
            Some(latest),
            |_, _| panic!("Should not bootstrap"),
            |_, timestamp| {
                assert_eq!(timestamp, "2026-02-05T00:00:00Z");
                Ok(Some("base_sha".to_string()))
            },
        )
        .unwrap();

        assert_eq!(result.selection_mode, "incremental");
        assert_eq!(result.from_commit, "base_sha");
        assert_eq!(result.to_commit, head);
        assert_eq!(result.changes_since.as_deref(), Some("2026-02-05T00:00:00Z"));
    }

    #[test]
    fn test_determine_range_strategy_bootstrap_no_file() {
        let head = "head_sha";
        let result = determine_range_strategy(
            head,
            None,
            |_, _| Ok("bootstrap_sha".to_string()),
            |_, _| panic!("Should not resolve incremental cursor"),
        )
        .unwrap();

        assert_eq!(result.selection_mode, "bootstrap");
        assert_eq!(result.from_commit, "bootstrap_sha");
        assert_eq!(result.to_commit, head);
        assert_eq!(result.changes_since, None);
    }

    #[test]
    fn test_determine_range_strategy_bootstrap_when_no_commit_before_cursor() {
        let head = "head_sha";
        let latest = r#"
created_at: "2026-02-05T00:00:00Z"
"#;
        let result = determine_range_strategy(
            head,
            Some(latest),
            |_, _| Ok("bootstrap_sha".to_string()),
            |_, _| Ok(None),
        )
        .unwrap();

        assert_eq!(result.selection_mode, "bootstrap");
        assert_eq!(result.from_commit, "bootstrap_sha");
        assert!(result.selection_detail.contains("Fallback bootstrap"));
        assert_eq!(result.changes_since.as_deref(), Some("2026-02-05T00:00:00Z"));
    }

    #[test]
    fn test_extract_created_at_missing_is_error() {
        let content = r#"
range:
  to_commit: "abcdef123456"
"#;
        let err = extract_created_at(content).unwrap_err();
        assert!(err.to_string().contains("created_at"));
    }

    #[test]
    fn test_extract_created_at_invalid_format_is_error() {
        let content = r#"
created_at: "2026-02-05 00:00:00"
"#;
        let err = extract_created_at(content).unwrap_err();
        assert!(err.to_string().contains("RFC3339"));
    }

    // --- Tests from mock/narrator.rs ---

    #[allow(dead_code)]
    struct MustNotTouchGit;

    impl GitPort for MustNotTouchGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_head_sha");
        }

        fn get_current_branch(&self) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_current_branch");
        }

        fn commit_exists(&self, _sha: &str) -> bool {
            panic!("mock narrator no-op must not call commit_exists");
        }

        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_nth_ancestor");
        }

        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            panic!("mock narrator no-op must not call has_changes");
        }

        fn run_command(
            &self,
            _args: &[&str],
            _cwd: Option<&std::path::Path>,
        ) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call run_command");
        }

        fn checkout_branch(&self, _branch: &str, _create: bool) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call checkout_branch");
        }

        fn push_branch(&self, _branch: &str, _force: bool) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call push_branch");
        }

        fn commit_files(
            &self,
            _message: &str,
            _files: &[&std::path::Path],
        ) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call commit_files");
        }

        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call fetch");
        }

        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            panic!("mock narrator no-op must not call delete_branch");
        }
    }

    #[allow(dead_code)]
    struct MustNotTouchGitHub;

    impl GitHubPort for MustNotTouchGitHub {
        fn create_pull_request(
            &self,
            _head: &str,
            _base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<PullRequestInfo, AppError> {
            panic!("mock narrator no-op must not call create_pull_request");
        }

        fn close_pull_request(&self, _pr_number: u64) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call close_pull_request");
        }

        fn delete_branch(&self, _branch: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call delete_branch");
        }

        fn create_issue(
            &self,
            _title: &str,
            _body: &str,
            _labels: &[&str],
        ) -> Result<crate::ports::IssueInfo, AppError> {
            panic!("mock narrator no-op must not call create_issue");
        }

        fn get_pr_detail(
            &self,
            _pr_number: u64,
        ) -> Result<crate::ports::PullRequestDetail, AppError> {
            panic!("mock narrator no-op must not call get_pr_detail");
        }
        fn list_pr_comments(
            &self,
            _pr_number: u64,
        ) -> Result<Vec<crate::ports::PrComment>, AppError> {
            panic!("mock narrator no-op must not call list_pr_comments");
        }
        fn create_pr_comment(&self, _pr_number: u64, _body: &str) -> Result<u64, AppError> {
            panic!("mock narrator no-op must not call create_pr_comment");
        }
        fn update_pr_comment(&self, _comment_id: u64, _body: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call update_pr_comment");
        }
        fn ensure_label(&self, _label: &str, _color: Option<&str>) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call ensure_label");
        }
        fn add_label_to_pr(&self, _pr_number: u64, _label: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call add_label_to_pr");
        }
        fn add_label_to_issue(&self, _issue_number: u64, _label: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call add_label_to_issue");
        }
        fn enable_automerge(&self, _pr_number: u64) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call enable_automerge");
        }
        fn list_pr_files(&self, _pr_number: u64) -> Result<Vec<String>, AppError> {
            panic!("mock narrator no-op must not call list_pr_files");
        }
    }

    #[allow(dead_code)]
    struct DummyWorkspace;

    impl crate::domain::PromptAssetLoader for DummyWorkspace {
        fn read_asset(&self, _path: &std::path::Path) -> std::io::Result<String> {
            panic!("mock narrator no-op must not call read_asset");
        }

        fn asset_exists(&self, _path: &std::path::Path) -> bool {
            panic!("mock narrator no-op must not call asset_exists");
        }

        fn ensure_asset_dir(&self, _path: &std::path::Path) -> std::io::Result<()> {
            panic!("mock narrator no-op must not call ensure_asset_dir");
        }

        fn copy_asset(
            &self,
            _from: &std::path::Path,
            _to: &std::path::Path,
        ) -> std::io::Result<u64> {
            panic!("mock narrator no-op must not call copy_asset");
        }
    }

    impl WorkspaceStore for DummyWorkspace {
        fn exists(&self) -> bool {
            panic!("mock narrator no-op must not call exists");
        }

        fn jlo_exists(&self) -> bool {
            panic!("mock narrator no-op must not call jlo_exists");
        }

        fn jules_path(&self) -> PathBuf {
            panic!("mock narrator no-op must not call jules_path");
        }

        fn jlo_path(&self) -> PathBuf {
            panic!("mock narrator no-op must not call jlo_path");
        }

        fn create_structure(&self, _scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call create_structure");
        }

        fn write_version(&self, _version: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call write_version");
        }

        fn read_version(&self) -> Result<Option<String>, AppError> {
            panic!("mock narrator no-op must not call read_version");
        }

        fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
            panic!("mock narrator no-op must not call discover_roles");
        }

        fn find_role_fuzzy(&self, _query: &str) -> Result<Option<DiscoveredRole>, AppError> {
            panic!("mock narrator no-op must not call find_role_fuzzy");
        }

        fn role_path(&self, _role: &DiscoveredRole) -> Option<PathBuf> {
            panic!("mock narrator no-op must not call role_path");
        }

        fn read_file(&self, _path: &str) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call read_file");
        }

        fn write_file(&self, _path: &str, _content: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call write_file");
        }

        fn remove_file(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call remove_file");
        }

        fn list_dir(&self, _path: &str) -> Result<Vec<PathBuf>, AppError> {
            panic!("mock narrator no-op must not call list_dir");
        }

        fn set_executable(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call set_executable");
        }

        fn file_exists(&self, _path: &str) -> bool {
            panic!("mock narrator no-op must not call file_exists");
        }

        fn is_dir(&self, _path: &str) -> bool {
            panic!("mock narrator no-op must not call is_dir");
        }

        fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call create_dir_all");
        }

        fn resolve_path(&self, _path: &str) -> PathBuf {
            panic!("mock narrator no-op must not call resolve_path");
        }

        fn canonicalize(&self, _path: &str) -> Result<PathBuf, AppError> {
            panic!("mock narrator no-op must not call canonicalize");
        }
    }

    #[test]
    fn narrator_mock_is_noop() {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Narrator, "jules-narrator-".to_string());
        let config = MockConfig {
            mock_tag: "mock-run-123".to_string(),
            branch_prefixes: prefixes,
            jlo_target_branch: "main".to_string(),
            jules_worker_branch: "jules".to_string(),
            issue_labels: vec!["bugs".to_string()],
        };

        let output = execute_mock(&config).expect("mock narrator should succeed as no-op");

        assert_eq!(output.mock_branch, "");
        assert_eq!(output.mock_pr_number, 0);
        assert_eq!(output.mock_pr_url, "");
        assert_eq!(output.mock_tag, "mock-run-123");
    }
}
