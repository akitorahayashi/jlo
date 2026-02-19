use std::path::Path;

use crate::app::commands::run::RunRuntimeOptions;
use crate::app::commands::run::input::{detect_repository_source, load_mock_config};
use crate::domain::layers::execute::starting_branch::resolve_starting_branch;
use crate::domain::prompt_assemble::{PromptAssetLoader, PromptContext, assemble_prompt};
use crate::domain::{AppError, ControlPlaneConfig, Layer, MockConfig, MockOutput, RunOptions};
use crate::ports::{
    AutomationMode, Git, GitHub, JloStore, JulesStore, RepositoryFilesystem, SessionRequest,
};

use super::super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct NarratorLayer;

impl<W> LayerStrategy<W> for NarratorLayer
where
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        _target: &RunOptions,
        runtime: &RunRuntimeOptions,
        config: &ControlPlaneConfig,
        git: &dyn Git,
        _github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if runtime.mock {
            let mock_config = load_mock_config(jules_path, repository)?;
            let output = execute_mock(&mock_config)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::super::mock::mock_execution::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::super::mock::mock_execution::print_local(&output);
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
            runtime.prompt_preview,
            runtime.branch.as_deref(),
            config,
            git,
            repository,
            client_factory,
        )
    }
}

/// Execute the Narrator layer in real mode.
fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    config: &ControlPlaneConfig,
    git: &G,
    repository: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
    G: Git + ?Sized,
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    let starting_branch = resolve_starting_branch(Layer::Narrator, config, branch);

    // Determine commit range
    let range = determine_range(git)?;

    let prompt = assemble_narrator_prompt(jules_path, &range, repository)?;

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
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
>(
    jules_path: &Path,
    range: &RangeContext,
    repository: &W,
) -> Result<String, AppError> {
    let prompt_context =
        PromptContext::new().with_var("range_description", build_range_description(range));

    let (prompt, seed_ops) = assemble_prompt(
        jules_path,
        Layer::Narrator,
        &prompt_context,
        repository,
        crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset,
    )
    .map_err(|e| AppError::InternalError(e.to_string()))?;
    super::execute_seed_ops(seed_ops, repository)?;
    Ok(prompt.content)
}

// --- Range Logic ---

/// Number of commits to summarize for narrator.
pub const BOOTSTRAP_COMMIT_COUNT: usize = 20;

#[derive(Debug, PartialEq)]
struct RangeContext {
    from_commit: String,
    to_commit: String,
}

fn determine_range<G>(git: &G) -> Result<RangeContext, AppError>
where
    G: Git + ?Sized,
{
    let head_sha = git.get_head_sha()?;
    determine_range_strategy(&head_sha, |sha, n| match git.get_nth_ancestor(sha, n)? {
        Some(commit) => Ok(commit),
        None => git.get_first_commit(sha),
    })
}

fn determine_range_strategy(
    head_sha: &str,
    get_bootstrap_commit: impl Fn(&str, usize) -> Result<String, AppError>,
) -> Result<RangeContext, AppError> {
    let bootstrap_from = get_bootstrap_commit(head_sha, BOOTSTRAP_COMMIT_COUNT)?;
    Ok(RangeContext { from_commit: bootstrap_from, to_commit: head_sha.to_string() })
}

fn build_range_description(range: &RangeContext) -> String {
    let short_from = &range.from_commit[..7.min(range.from_commit.len())];
    let short_to = &range.to_commit[..7.min(range.to_commit.len())];
    format!(
        "Summarize the most recent {} commits with non-.jules/.jlo scope (commits {}..{}).",
        BOOTSTRAP_COMMIT_COUNT, short_from, short_to
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::GitWorkspace;
    use crate::ports::{
        DiscoveredRole, JloStore, JulesStore, PullRequestInfo, RepositoryFilesystem, ScaffoldFile,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    // --- Range strategy tests ---

    #[test]
    fn test_determine_range_strategy_recent_window() {
        let head = "head_sha";
        let result = determine_range_strategy(head, |sha, count| {
            assert_eq!(sha, head);
            assert_eq!(count, BOOTSTRAP_COMMIT_COUNT);
            Ok("bootstrap_sha".to_string())
        })
        .unwrap();

        assert_eq!(result.from_commit, "bootstrap_sha");
        assert_eq!(result.to_commit, head);
    }

    #[test]
    fn test_build_range_description_mentions_recent_window() {
        let range = RangeContext {
            from_commit: "0123456789abcdef".to_string(),
            to_commit: "fedcba9876543210".to_string(),
        };
        let description = build_range_description(&range);
        assert!(description.contains(&BOOTSTRAP_COMMIT_COUNT.to_string()));
        assert!(description.contains("0123456"));
        assert!(description.contains("fedcba9"));
    }

    // --- Tests from mock/narrator.rs ---

    #[allow(dead_code)]
    struct MustNotTouchGit;

    impl Git for MustNotTouchGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_head_sha");
        }

        fn get_current_branch(&self) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_current_branch");
        }

        fn commit_exists(&self, _sha: &str) -> bool {
            panic!("mock narrator no-op must not call commit_exists");
        }

        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<Option<String>, AppError> {
            panic!("mock narrator no-op must not call get_nth_ancestor");
        }

        fn get_first_commit(&self, _commit: &str) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_first_commit");
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

        fn push_branch_from_rev(
            &self,
            _rev: &str,
            _branch: &str,
            _force: bool,
        ) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call push_branch_from_rev");
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

        fn create_workspace(&self, _branch: &str) -> Result<Box<dyn GitWorkspace>, AppError> {
            panic!("mock narrator no-op must not call create_workspace");
        }
    }

    #[allow(dead_code)]
    struct MustNotTouchGitHub;

    impl GitHub for MustNotTouchGitHub {
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
        fn merge_pull_request(&self, _pr_number: u64) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call merge_pull_request");
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

    impl RepositoryFilesystem for DummyWorkspace {
        fn read_file(&self, _path: &str) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call read_file");
        }

        fn write_file(&self, _path: &str, _content: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call write_file");
        }

        fn remove_file(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call remove_file");
        }

        fn remove_dir_all(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call remove_dir_all");
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

    impl JloStore for DummyWorkspace {
        fn jlo_exists(&self) -> bool {
            panic!("mock narrator no-op must not call jlo_exists");
        }

        fn jlo_path(&self) -> PathBuf {
            panic!("mock narrator no-op must not call jlo_path");
        }

        fn jlo_write_version(&self, _version: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call jlo_write_version");
        }

        fn jlo_read_version(&self) -> Result<Option<String>, AppError> {
            panic!("mock narrator no-op must not call jlo_read_version");
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

        fn write_role(
            &self,
            _layer: Layer,
            _role_id: &str,
            _content: &str,
        ) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call write_role");
        }
    }

    impl JulesStore for DummyWorkspace {
        fn jules_exists(&self) -> bool {
            panic!("mock narrator no-op must not call jules_exists");
        }

        fn jules_path(&self) -> PathBuf {
            panic!("mock narrator no-op must not call jules_path");
        }

        fn jules_write_version(&self, _version: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call jules_write_version");
        }

        fn jules_read_version(&self) -> Result<Option<String>, AppError> {
            panic!("mock narrator no-op must not call jules_read_version");
        }

        fn create_structure(&self, _scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call create_structure");
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
