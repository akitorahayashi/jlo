//! Run command implementation for executing Jules agents.

mod input;
mod layer;
mod mock;
mod role_session;
mod strategy;

use std::path::Path;

use crate::adapters::jules_client::HttpJulesClient;
use crate::adapters::jules_client::{RetryPolicy, RetryingJulesClient};
use crate::app::commands::run::input::{load_control_plane_config, validate_mock_prerequisites};
use crate::app::commands::run::strategy::{JulesClientFactory, get_layer_strategy};
use crate::app::commands::workflow::exchange::{
    ExchangeCleanRequirementOptions, clean_requirement_apply_with_adapters,
};
use crate::app::commands::workflow::push::{
    PushWorkerBranchOptions, execute as push_worker_branch,
};
use crate::domain::PromptAssetLoader;
pub use crate::domain::RunOptions;
use crate::domain::validation::validate_identifier;
use crate::domain::{AppError, JulesApiConfig};
use crate::ports::{Git, GitHub, JloStore, JulesClient, JulesStore, RepositoryFilesystem};

pub use strategy::RunResult;

/// Runtime execution context for the run command.
#[derive(Debug, Clone, Default)]
pub struct RunRuntimeOptions {
    /// Show assembled prompts without executing.
    pub prompt_preview: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Run in mock mode (no Jules API, real git/GitHub operations).
    pub mock: bool,
    /// Skip post-execution cleanup (requirement deletion and worker-branch push).
    pub no_cleanup: bool,
}

struct LazyClientFactory {
    config: JulesApiConfig,
}

impl JulesClientFactory for LazyClientFactory {
    fn create(&self) -> Result<Box<dyn JulesClient>, AppError> {
        let transport = HttpJulesClient::from_env_with_config(&self.config)?;
        let retry_policy = RetryPolicy::from_config(&self.config);
        Ok(Box::new(RetryingJulesClient::new(Box::new(transport), retry_policy)))
    }
}

/// Execute the run command.
pub fn execute<G, H, W>(
    jules_path: &Path,
    target: RunOptions,
    runtime: RunRuntimeOptions,
    git: &G,
    github: &H,
    repository: &W,
) -> Result<RunResult, AppError>
where
    G: Git,
    H: GitHub,
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    execute_with_mock_prerequisite_validator(
        jules_path,
        target,
        runtime,
        git,
        github,
        repository,
        validate_mock_prerequisites,
    )
}

fn execute_with_mock_prerequisite_validator<G, H, W, F>(
    jules_path: &Path,
    target: RunOptions,
    runtime: RunRuntimeOptions,
    git: &G,
    github: &H,
    repository: &W,
    validate_mock: F,
) -> Result<RunResult, AppError>
where
    G: Git,
    H: GitHub,
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
    F: Fn() -> Result<(), AppError>,
{
    // Validate task selector if provided (prevents path traversal)
    if let Some(ref task) = target.task
        && !validate_identifier(task, false)
    {
        return Err(AppError::Validation(format!(
            "Invalid task '{}': must be a safe path component (e.g. 'create_three_proposals')",
            task,
        )));
    }

    if target.task.is_some() && target.layer != crate::domain::Layer::Innovators {
        return Err(AppError::Validation(
            "--task is only supported when layer is innovators".to_string(),
        ));
    }

    // Load configuration
    let config = load_control_plane_config(jules_path, repository)?;

    let expected_branch = if target.layer.uses_worker_branch() {
        config.run.jules_worker_branch.as_str()
    } else {
        config.run.jlo_target_branch.as_str()
    };

    // Validate current branch matches the layer's branch contract.
    // --branch override bypasses this check (explicit operator override).
    if runtime.branch.is_none() {
        let current = git.get_current_branch()?;
        if current != expected_branch {
            return Err(AppError::Validation(format!(
                "Layer '{}' requires branch '{}', but current branch is '{}'",
                target.layer.dir_name(),
                expected_branch,
                current,
            )));
        }
    }

    if runtime.mock {
        validate_mock()?;
    }

    // Create client factory
    let client_factory = LazyClientFactory { config: config.jules_api.clone() };

    // Get layer strategy
    let strategy = get_layer_strategy(target.layer);

    // Execute strategy
    let result = strategy.execute(
        jules_path,
        &target,
        &runtime,
        &config,
        git,
        github,
        repository,
        &client_factory,
    )?;

    // Mock executions can checkout ephemeral branches during simulation.
    // Restore the expected layer branch so subsequent runs keep branch context.
    if runtime.mock && runtime.branch.is_none() {
        git.checkout_branch(expected_branch, false)?;
    }

    // Handle post-execution cleanup (e.g. Implementer requirement)
    if !runtime.no_cleanup
        && let Some(path) = result.cleanup_requirement.as_ref()
    {
        let path_str = path.to_string_lossy().to_string();
        let cleanup_res = clean_requirement_apply_with_adapters(
            ExchangeCleanRequirementOptions { requirement_file: path_str },
            repository,
            git,
        )?;
        println!(
            "✅ Cleaned requirement and source events ({} file(s) removed)",
            cleanup_res.deleted_paths.len()
        );

        if !runtime.mock {
            push_worker_branch(PushWorkerBranchOptions {
                change_token: format!("requirement-cleanup-{}", cleanup_res.requirement_id),
                commit_message: format!("jules: clean requirement {}", cleanup_res.requirement_id),
                pr_title: format!("chore: clean requirement {}", cleanup_res.requirement_id),
                pr_body: format!(
                    "Automated cleanup for processed requirement `{}`.\n\n- remove requirement artifact\n- remove source event artifacts",
                    cleanup_res.requirement_id
                ),
            })?;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::local_repository::LocalRepositoryAdapter;
    use crate::ports::{
        GitHub, IssueInfo, JulesStore, PrComment, PullRequestDetail, PullRequestInfo,
    };
    use serial_test::serial;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    #[derive(Clone)]
    struct TestGit {
        root: PathBuf,
        current_branch: Arc<Mutex<String>>,
        pushed_branches: Arc<Mutex<Vec<String>>>,
        commit_counter: Arc<Mutex<u64>>,
    }

    impl TestGit {
        fn new(root: PathBuf, initial_branch: &str) -> Self {
            Self {
                root,
                current_branch: Arc::new(Mutex::new(initial_branch.to_string())),
                pushed_branches: Arc::new(Mutex::new(Vec::new())),
                commit_counter: Arc::new(Mutex::new(0)),
            }
        }

        fn pushed_branches(&self) -> Vec<String> {
            self.pushed_branches.lock().expect("push lock poisoned").clone()
        }
    }

    impl Git for TestGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            let counter = *self.commit_counter.lock().expect("counter lock poisoned");
            Ok(format!("mocksha{:06}", counter))
        }

        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok(self.current_branch.lock().expect("branch lock poisoned").clone())
        }

        fn commit_exists(&self, _sha: &str) -> bool {
            true
        }

        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<Option<String>, AppError> {
            Ok(Some("mocksha000000".to_string()))
        }

        fn get_first_commit(&self, _commit: &str) -> Result<String, AppError> {
            Ok("mocksha000000".to_string())
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
            if args.len() >= 3 && args[0] == "rm" && args[1] == "--" {
                for rel_path in &args[2..] {
                    let path = self.root.join(rel_path);
                    if path.exists() {
                        fs::remove_file(path)?;
                    }
                }
                return Ok(String::new());
            }

            if !args.is_empty() && args[0] == "commit" {
                let mut counter = self.commit_counter.lock().expect("counter lock poisoned");
                *counter += 1;
                return Ok(format!("mocksha{:06}", *counter));
            }

            Ok(String::new())
        }

        fn checkout_branch(&self, branch: &str, _create: bool) -> Result<(), AppError> {
            let normalized = branch.strip_prefix("origin/").unwrap_or(branch).to_string();
            *self.current_branch.lock().expect("branch lock poisoned") = normalized;
            Ok(())
        }

        fn push_branch(&self, branch: &str, _force: bool) -> Result<(), AppError> {
            self.pushed_branches.lock().expect("push lock poisoned").push(branch.to_string());
            Ok(())
        }

        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            let mut counter = self.commit_counter.lock().expect("counter lock poisoned");
            *counter += 1;
            Ok(format!("mocksha{:06}", *counter))
        }

        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    struct TestGitHub {
        pr_counter: Arc<Mutex<u64>>,
    }

    impl TestGitHub {
        fn new() -> Self {
            Self { pr_counter: Arc::new(Mutex::new(1)) }
        }
    }

    impl GitHub for TestGitHub {
        fn create_pull_request(
            &self,
            head: &str,
            base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<PullRequestInfo, AppError> {
            let mut counter = self.pr_counter.lock().expect("pr lock poisoned");
            let number = *counter;
            *counter += 1;
            Ok(PullRequestInfo {
                number,
                url: format!("https://example.com/pr/{}", number),
                head: head.to_string(),
                base: base.to_string(),
            })
        }

        fn close_pull_request(&self, _pr_number: u64) -> Result<(), AppError> {
            Ok(())
        }

        fn delete_branch(&self, _branch: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn create_issue(
            &self,
            _title: &str,
            _body: &str,
            _labels: &[&str],
        ) -> Result<IssueInfo, AppError> {
            Ok(IssueInfo { number: 1, url: "https://example.com/issues/1".to_string() })
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

        fn merge_pull_request(&self, _pr_number: u64) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(serde::Deserialize)]
    struct RequirementDoc {
        id: String,
        implementation_ready: bool,
        source_events: Vec<String>,
    }

    #[derive(serde::Deserialize)]
    struct EventDoc {
        requirement_id: String,
    }

    struct EnvVarGuard {
        key: String,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set<K: Into<String>, V: AsRef<std::ffi::OsStr>>(key: K, value: V) -> Self {
            let key = key.into();
            let original = std::env::var_os(&key);
            // SAFETY: This helper is used only from serial tests in this module.
            // No concurrent environment access occurs while the guard is alive.
            unsafe {
                std::env::set_var(&key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(original) = self.original.as_ref() {
                // SAFETY: Drop runs in the same serial-test context as `set`.
                unsafe {
                    std::env::set_var(&self.key, original);
                }
            } else {
                // SAFETY: Drop runs in the same serial-test context as `set`.
                unsafe {
                    std::env::remove_var(&self.key);
                }
            }
        }
    }

    fn write_mock_workspace(root: &Path, mock_tag: &str) {
        fs::create_dir_all(root.join(".jules/exchange/events/pending"))
            .expect("create pending dir");
        fs::create_dir_all(root.join(".jules/exchange/requirements"))
            .expect("create requirements dir");
        // Create schemas directories for layers that have schemas
        for layer in crate::domain::Layer::ALL {
            if layer.has_schemas() {
                fs::create_dir_all(root.join(format!(".jules/schemas/{}", layer.dir_name())))
                    .expect("create schemas dir");
            }
        }
        fs::create_dir_all(root.join(".jlo/roles/observers/taxonomy"))
            .expect("create observer role dir");

        fs::write(
            root.join(".jlo/config.toml"),
            r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
"#,
        )
        .expect("write config");
        fs::write(root.join(".jlo/roles/observers/taxonomy/role.yml"), "id: taxonomy\n")
            .expect("write observer role");

        fs::write(
            root.join(".jules/github-labels.json"),
            r#"{"issue_labels":{"bugs":{"color":"d73a4a"}}}"#,
        )
        .expect("write labels");

        let event_ids = ["aa1111", "bb2222", "cc3333", "dd4444"];
        for event_id in event_ids {
            let path =
                root.join(format!(".jules/exchange/events/pending/{}-{}.yml", mock_tag, event_id));
            fs::write(path, format!("id: {}\nsummary: mock event {}\n", event_id, event_id))
                .expect("write pending event");
        }
    }

    fn read_requirement_doc(path: &Path) -> RequirementDoc {
        let content = fs::read_to_string(path).expect("read requirement");
        serde_yaml::from_str::<RequirementDoc>(&content).expect("parse requirement")
    }

    fn read_event_doc(path: &Path) -> EventDoc {
        let content = fs::read_to_string(path).expect("read event");
        serde_yaml::from_str::<EventDoc>(&content).expect("parse event")
    }

    #[test]
    #[serial]
    fn mock_decider_and_implementer_cleanup_tracks_source_events_consistently() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        let mock_tag = "mock-run-unit";

        write_mock_workspace(&root, mock_tag);

        let _mock_tag_env = EnvVarGuard::set("JULES_MOCK_TAG", mock_tag);

        let repository = LocalRepositoryAdapter::new(root.clone());
        let github = TestGitHub::new();

        let decider_git = TestGit::new(root.clone(), "jules");
        execute_with_mock_prerequisite_validator(
            &repository.jules_path(),
            RunOptions {
                layer: crate::domain::Layer::Decider,
                role: None,
                requirement: None,
                task: None,
            },
            RunRuntimeOptions {
                prompt_preview: false,
                branch: None,
                mock: true,
                no_cleanup: false,
            },
            &decider_git,
            &github,
            &repository,
            || Ok(()),
        )
        .expect("decider run should succeed");

        let mut requirement_files: Vec<PathBuf> =
            fs::read_dir(root.join(".jules/exchange/requirements"))
                .expect("read requirements dir")
                .map(|entry| entry.expect("read dir entry").path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "yml"))
                .collect();
        requirement_files.sort();
        assert_eq!(requirement_files.len(), 2, "decider should create two requirements");

        let mut all_source_events: Vec<String> = Vec::new();
        let mut source_events_by_requirement: HashMap<String, Vec<String>> = HashMap::new();
        let mut implementer_requirement: Option<PathBuf> = None;
        let mut planner_requirement: Option<PathBuf> = None;

        for requirement_path in &requirement_files {
            let requirement = read_requirement_doc(requirement_path);
            if !requirement.implementation_ready {
                planner_requirement = Some(requirement_path.clone());
            } else {
                implementer_requirement = Some(requirement_path.clone());
            }

            all_source_events.extend(requirement.source_events.clone());
            source_events_by_requirement.insert(requirement.id, requirement.source_events);
        }

        let implementer_requirement =
            implementer_requirement.expect("implementer requirement should exist");
        let planner_requirement = planner_requirement.expect("planner requirement should exist");

        all_source_events.sort();
        assert_eq!(
            all_source_events,
            vec![
                "aa1111".to_string(),
                "bb2222".to_string(),
                "cc3333".to_string(),
                "dd4444".to_string()
            ],
            "all decided events should be assigned to requirement source_events"
        );

        let mut source_sizes: Vec<usize> =
            source_events_by_requirement.values().map(std::vec::Vec::len).collect();
        source_sizes.sort();
        assert_eq!(
            source_sizes,
            vec![1, 3],
            "decider split should represent a 1-event planner route and 3-event implementer route"
        );

        let event_ids = ["aa1111", "bb2222", "cc3333", "dd4444"];
        for event_id in event_ids {
            let decided_path =
                root.join(format!(".jules/exchange/events/decided/{}-{}.yml", mock_tag, event_id));
            assert!(
                decided_path.exists(),
                "decided event should exist: {}",
                decided_path.display()
            );

            let event_doc = read_event_doc(&decided_path);
            assert!(
                source_events_by_requirement
                    .get(&event_doc.requirement_id)
                    .is_some_and(|sources| sources.contains(&event_id.to_string())),
                "event {} must belong to exactly one requirement source_events owner",
                event_id
            );
        }

        let implementer_requirement_doc = read_requirement_doc(&implementer_requirement);
        let planner_requirement_doc = read_requirement_doc(&planner_requirement);
        let implementer_sources = implementer_requirement_doc.source_events;
        let planner_sources = planner_requirement_doc.source_events;

        let implementer_git = TestGit::new(root.clone(), "main");
        execute_with_mock_prerequisite_validator(
            &repository.jules_path(),
            RunOptions {
                layer: crate::domain::Layer::Implementer,
                role: None,
                requirement: Some(implementer_requirement.clone()),
                task: None,
            },
            RunRuntimeOptions {
                prompt_preview: false,
                branch: None,
                mock: true,
                no_cleanup: false,
            },
            &implementer_git,
            &github,
            &repository,
            || Ok(()),
        )
        .expect("implementer run should succeed");

        assert!(
            !implementer_requirement.exists(),
            "implementer requirement should be deleted by post-run cleanup"
        );
        assert!(
            planner_requirement.exists(),
            "planner requirement should remain after implementer cleanup"
        );

        for event_id in &implementer_sources {
            let path =
                root.join(format!(".jules/exchange/events/decided/{}-{}.yml", mock_tag, event_id));
            assert!(
                !path.exists(),
                "implementer-owned source event should be deleted during cleanup: {}",
                path.display()
            );
        }

        for event_id in &planner_sources {
            let path =
                root.join(format!(".jules/exchange/events/decided/{}-{}.yml", mock_tag, event_id));
            assert!(path.exists(), "planner-owned source event should remain: {}", path.display());
        }

        let pushed = implementer_git.pushed_branches();
        assert!(
            pushed.iter().any(|branch| branch.starts_with("jules-implementer-")),
            "implementer mock should push an implementer branch"
        );
        assert!(
            !pushed.iter().any(|branch| branch == "jules"),
            "mock implementer cleanup should not run worker-branch merge push"
        );
    }

    #[test]
    #[serial]
    fn wrong_branch_fails_fast_for_worker_branch_layer() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        write_mock_workspace(&root, "test");

        let repository = LocalRepositoryAdapter::new(root.clone());
        let github = TestGitHub::new();
        // Decider expects worker branch "jules", but current is "main"
        let git = TestGit::new(root.clone(), "main");

        let result = execute_with_mock_prerequisite_validator(
            &repository.jules_path(),
            RunOptions {
                layer: crate::domain::Layer::Decider,
                role: None,
                requirement: None,
                task: None,
            },
            RunRuntimeOptions {
                prompt_preview: false,
                branch: None,
                mock: false,
                no_cleanup: false,
            },
            &git,
            &github,
            &repository,
            || Ok(()),
        );

        let err = result.expect_err("should fail on wrong branch");
        let msg = err.to_string();
        assert!(msg.contains("decider"), "error should name the layer: {}", msg);
        assert!(msg.contains("jules"), "error should name expected branch: {}", msg);
        assert!(msg.contains("main"), "error should name current branch: {}", msg);
    }

    #[test]
    #[serial]
    fn wrong_branch_fails_fast_for_target_branch_layer() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        write_mock_workspace(&root, "test");

        let repository = LocalRepositoryAdapter::new(root.clone());
        let github = TestGitHub::new();
        // Implementer expects target branch "main", but current is "jules"
        let git = TestGit::new(root.clone(), "jules");

        let result = execute_with_mock_prerequisite_validator(
            &repository.jules_path(),
            RunOptions {
                layer: crate::domain::Layer::Implementer,
                role: None,
                requirement: Some(root.join(".jules/exchange/requirements/fake.yml")),
                task: None,
            },
            RunRuntimeOptions {
                prompt_preview: false,
                branch: None,
                mock: false,
                no_cleanup: false,
            },
            &git,
            &github,
            &repository,
            || Ok(()),
        );

        let err = result.expect_err("should fail on wrong branch");
        let msg = err.to_string();
        assert!(msg.contains("implementer"), "error should name the layer: {}", msg);
        assert!(msg.contains("main"), "error should name expected branch: {}", msg);
        assert!(msg.contains("jules"), "error should name current branch: {}", msg);
    }

    #[test]
    #[serial]
    fn branch_override_bypasses_validation() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        write_mock_workspace(&root, "test");

        let repository = LocalRepositoryAdapter::new(root.clone());
        let github = TestGitHub::new();
        // Narrator expects worker branch "jules", but current is "feature"
        // --branch override should bypass the check
        let git = TestGit::new(root.clone(), "feature");

        let _mock_tag_env = EnvVarGuard::set("JULES_MOCK_TAG", "mock-test");

        let result = execute_with_mock_prerequisite_validator(
            &repository.jules_path(),
            RunOptions {
                layer: crate::domain::Layer::Narrator,
                role: None,
                requirement: None,
                task: None,
            },
            RunRuntimeOptions {
                prompt_preview: false,
                branch: Some("custom-branch".to_string()),
                mock: true,
                no_cleanup: false,
            },
            &git,
            &github,
            &repository,
            || Ok(()),
        );

        // With --branch override, branch validation is skipped.
        // The mock narrator should proceed past validation.
        assert!(result.is_ok(), "branch override should bypass validation: {:?}", result.err());
    }

    #[test]
    #[serial]
    fn mock_observer_restores_worker_branch_after_success() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        write_mock_workspace(&root, "mock-test");

        let repository = LocalRepositoryAdapter::new(root.clone());
        let github = TestGitHub::new();
        let git = TestGit::new(root.clone(), "jules");

        let _mock_tag_env = EnvVarGuard::set("JULES_MOCK_TAG", "mock-test");

        let result = execute_with_mock_prerequisite_validator(
            &repository.jules_path(),
            RunOptions {
                layer: crate::domain::Layer::Observers,
                role: Some("taxonomy".to_string()),
                requirement: None,
                task: None,
            },
            RunRuntimeOptions {
                prompt_preview: false,
                branch: None,
                mock: true,
                no_cleanup: false,
            },
            &git,
            &github,
            &repository,
            || Ok(()),
        );

        assert!(result.is_ok(), "mock observer run should succeed: {:?}", result.err());
        assert_eq!(
            git.get_current_branch().expect("branch should be readable"),
            "jules",
            "mock observer run should restore worker branch context"
        );
    }
}
