use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Deserialize;

use crate::domain::configuration::loader::detect_repository_source;
use crate::domain::configuration::mock_loader::load_mock_config;
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, MockConfig, MockOutput, RunConfig, RunOptions};
use crate::ports::{
    AutomationMode, GitHubPort, GitPort, JulesClient, SessionRequest, WorkspaceStore,
};

use super::requirement::validate_requirement_path;
use super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct ImplementerLayer;

impl<W> LayerStrategy<W> for ImplementerLayer
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn GitPort,
        github: &dyn GitHubPort,
        workspace: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if options.mock {
            let mock_config = load_mock_config(jules_path, options, workspace)?;
            let output = execute_mock(jules_path, options, &mock_config, git, github, workspace)?;
            let cleanup_requirement = options.requirement.clone();
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::mock_utils::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::mock_utils::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec!["implementer".to_string()],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement,
            });
        }

        execute_real(
            jules_path,
            options.prompt_preview,
            options.branch.as_deref(),
            options.requirement.as_deref(),
            config,
            git,
            workspace,
            client_factory,
        )
    }
}

fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    requirement_path: Option<&Path>,
    config: &RunConfig,
    git: &G,
    workspace: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
    G: GitPort + ?Sized,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let requirement_path = requirement_path.ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for implementer".to_string())
    })?;
    let requirement_info = validate_requirement_path(requirement_path, workspace)?;

    let requirement_content = workspace.read_file(&requirement_info.requirement_path_str)?;

    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.default_branch.clone());

    if prompt_preview {
        execute_prompt_preview(jules_path, &starting_branch, &requirement_content, workspace)?;
        return Ok(RunResult {
            roles: vec!["implementer".to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let source = detect_repository_source(git)?;
    let client = client_factory.create()?;

    let session_id = execute_session(
        jules_path,
        &starting_branch,
        &source,
        client.as_ref(),
        &requirement_content,
        workspace,
    )?;

    // Return cleanup requirement path so caller can clean it up
    Ok(RunResult {
        roles: vec!["implementer".to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
        cleanup_requirement: Some(PathBuf::from(requirement_info.requirement_path_str)),
    })
}

fn execute_session<C: JulesClient + ?Sized, W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    starting_branch: &str,
    source: &str,
    client: &C,
    requirement_content: &str,
    workspace: &W,
) -> Result<String, AppError> {
    println!("Executing {}...", Layer::Implementer.display_name());

    let mut prompt = assemble_implementer_prompt(jules_path, requirement_content, workspace)?;

    prompt.push_str("\n---\n# Requirement Content\n");
    prompt.push_str(requirement_content);

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch: starting_branch.to_string(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(response.session_id)
}

fn assemble_implementer_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    requirement_content: &str,
    workspace: &W,
) -> Result<String, AppError> {
    let label = extract_requirement_label(requirement_content)?;
    let task_content = resolve_implementer_task(jules_path, &label, workspace)?;

    let context = PromptContext::new().with_var("task", task_content);

    assemble_prompt(jules_path, Layer::Implementer, &context, workspace)
        .map(|p: AssembledPrompt| p.content)
        .map_err(|e| AppError::InternalError(e.to_string()))
}

fn extract_requirement_label(requirement_content: &str) -> Result<String, AppError> {
    let value: serde_yaml::Value = serde_yaml::from_str(requirement_content)
        .map_err(|e| AppError::Validation(format!("Failed to parse requirement YAML: {}", e)))?;

    let label =
        value.get("label").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or_else(|| {
            AppError::Validation(
                "Requirement file must contain a non-empty 'label' field".to_string(),
            )
        })?;

    if !crate::domain::identifiers::validation::validate_safe_path_component(label) {
        return Err(AppError::Validation(format!(
            "Invalid label '{}': must be a safe path component",
            label
        )));
    }

    Ok(label.to_string())
}

fn resolve_implementer_task<W: WorkspaceStore>(
    jules_path: &Path,
    label: &str,
    workspace: &W,
) -> Result<String, AppError> {
    let task_path = jules::tasks_dir(jules_path, Layer::Implementer).join(format!("{}.yml", label));

    workspace.read_file(&task_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "No task file for label '{}': expected {}",
            label,
            task_path.display()
        ))
    })
}

fn execute_prompt_preview<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    starting_branch: &str,
    requirement_content: &str,
    workspace: &W,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", Layer::Implementer.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Requirement content: {} chars\n", requirement_content.len());

    let prompt_path = jules::prompt_template(jules_path, Layer::Implementer);
    let contracts_path = jules::contracts(jules_path, Layer::Implementer);

    println!("Prompt: {}", prompt_path.display());
    if contracts_path.exists() {
        println!("Contracts: {}", contracts_path.display());
    }

    let mut prompt = assemble_implementer_prompt(jules_path, requirement_content, workspace)?;
    prompt.push_str("\n---\n# Requirement Content\n");
    prompt.push_str(requirement_content);

    println!("Assembled prompt: {} chars (Prompt + No Path + Requirement Content)", prompt.len());

    println!("\nWould execute 1 session");
    Ok(())
}

fn execute_mock<G, H, W>(
    _jules_path: &Path,
    options: &RunOptions,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort + ?Sized,
    H: GitHubPort + ?Sized,
    W: WorkspaceStore,
{
    let original_branch = git.get_current_branch()?;

    let requirement_path = options.requirement.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for implementer".to_string())
    })?;

    // Parse requirement to get label and id
    let requirement_path_str = requirement_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid requirement path".to_string()))?;

    let requirement_content = workspace.read_file(requirement_path_str)?;
    let (label, issue_id) = parse_requirement_for_branch(&requirement_content, requirement_path)?;
    if !config.issue_labels.contains(&label) {
        return Err(AppError::Validation(format!(
            "Issue label '{}' is not defined in github-labels.json",
            label
        )));
    }

    // Implementer branch format: jules-implementer-<label>-<id>-<short_description>
    let prefix = config.branch_prefix(Layer::Implementer)?;
    let issue_id_short = issue_id.chars().take(6).collect::<String>();
    let branch_name = format!("{}{}-{}-{}", prefix, label, issue_id_short, config.mock_tag);

    println!("Mock implementer: creating branch {}", branch_name);

    // Fetch and checkout from default branch (not jules)
    git.fetch("origin")?;
    let base_branch = options.branch.as_deref().unwrap_or(&config.default_branch);
    git.checkout_branch(&format!("origin/{}", base_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create minimal mock file to have a commit
    let mock_file_path = format!(".mock-{}", config.mock_tag);
    let mock_content = format!(
        "# Mock implementation marker\n# Mock tag: {}\n# Issue: {}\n# Created: {}\n",
        config.mock_tag,
        issue_id,
        Utc::now().to_rfc3339()
    );

    workspace.write_file(&mock_file_path, &mock_content)?;

    // Commit and push
    let mock_path = Path::new(&mock_file_path);
    let files: Vec<&Path> = vec![mock_path];
    git.commit_files(&format!("[{}] implementer: mock implementation", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR targeting default branch (NOT jules)
    let pr = github.create_pull_request(
        &branch_name,
        base_branch,
        &format!("[{}] Implementation: {}", config.mock_tag, label),
        &format!(
            "Mock implementer run for workflow validation.\n\nMock tag: `{}`\nIssue: `{}`\nLabel: `{}`\n\n⚠️ This PR targets `{}` (not `jules`) - requires human review.",
            config.mock_tag,
            issue_id,
            label,
            base_branch
        ),
    )?;

    // NOTE: Implementer PRs do NOT get auto-merge enabled
    println!("Mock implementer: created PR #{} ({}) - awaiting label", pr.number, pr.url);

    // Restore original branch so post-run cleanup (requirement + source events) runs on
    // the exchange-bearing branch instead of the implementer branch.
    let restore_branch = if original_branch.trim().is_empty() {
        config.jules_branch.as_str()
    } else {
        &original_branch
    };
    git.checkout_branch(restore_branch, false)?;

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}

fn parse_requirement_for_branch(content: &str, path: &Path) -> Result<(String, String), AppError> {
    #[derive(Deserialize)]
    struct RequirementMeta {
        label: Option<String>,
        id: Option<String>,
    }

    let parsed: RequirementMeta = serde_yaml::from_str(content).map_err(|err| {
        AppError::Validation(format!(
            "Requirement file must be valid YAML ({}): {}",
            path.display(),
            err
        ))
    })?;

    let label = parsed.label.filter(|value| !value.trim().is_empty()).ok_or_else(|| {
        AppError::Validation(format!("Requirement file missing label field: {}", path.display()))
    })?;
    if !crate::domain::identifiers::validation::validate_safe_path_component(&label) {
        return Err(AppError::Validation(format!(
            "Requirement label '{}' is not a safe path component: {}",
            label,
            path.display()
        )));
    }

    let id = parsed.id.filter(|value| !value.trim().is_empty()).ok_or_else(|| {
        AppError::Validation(format!("Requirement file missing id field: {}", path.display()))
    })?;

    if id.len() != 6 || !id.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit()) {
        return Err(AppError::Validation(format!(
            "Issue id must be 6 lowercase alphanumeric chars: {}",
            path.display()
        )));
    }

    Ok((label, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockWorkspaceStore;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeGit {
        committed_files: Mutex<Vec<PathBuf>>,
    }

    impl FakeGit {
        fn new() -> Self {
            Self { committed_files: Mutex::new(vec![]) }
        }
    }

    impl GitPort for FakeGit {
        fn get_head_sha(&self) -> Result<String, AppError> { Ok("abc123".into()) }
        fn get_current_branch(&self) -> Result<String, AppError> { Ok("jules".into()) }
        fn commit_exists(&self, _sha: &str) -> bool { true }
        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> { Ok("parent".into()) }
        fn has_changes(&self, _from: &str, _to: &str, _pathspec: &[&str]) -> Result<bool, AppError> { Ok(false) }
        fn run_command(&self, _args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> { Ok(String::new()) }
        fn fetch(&self, _remote: &str) -> Result<(), AppError> { Ok(()) }
        fn checkout_branch(&self, _name: &str, _create: bool) -> Result<(), AppError> { Ok(()) }
        fn push_branch(&self, _name: &str, _force: bool) -> Result<(), AppError> { Ok(()) }
        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> { Ok(true) }
        fn commit_files(&self, _msg: &str, files: &[&Path]) -> Result<String, AppError> {
            let mut committed = self.committed_files.lock().unwrap();
            for f in files {
                committed.push(f.to_path_buf());
            }
            Ok("fake-sha".into())
        }
    }

    struct FakeGitHub;

    impl GitHubPort for FakeGitHub {
        fn create_pull_request(
            &self,
            head: &str,
            base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<crate::ports::PullRequestInfo, AppError> {
            Ok(crate::ports::PullRequestInfo {
                number: 202,
                url: "https://example.com/pr/202".into(),
                head: head.to_string(),
                base: base.to_string(),
            })
        }
        fn close_pull_request(&self, _pr_number: u64) -> Result<(), AppError> { Ok(()) }
        fn delete_branch(&self, _branch: &str) -> Result<(), AppError> { Ok(()) }
        fn create_issue(&self, _title: &str, _body: &str, _labels: &[&str]) -> Result<crate::ports::IssueInfo, AppError> {
            Ok(crate::ports::IssueInfo { number: 1, url: "https://example.com/issues/1".into() })
        }
        fn get_pr_detail(&self, _pr_number: u64) -> Result<crate::ports::PullRequestDetail, AppError> {
             Ok(crate::ports::PullRequestDetail { number: 202, head: String::new(), base: String::new(), is_draft: false, auto_merge_enabled: false })
        }
        fn list_pr_comments(&self, _pr_number: u64) -> Result<Vec<crate::ports::PrComment>, AppError> { Ok(Vec::new()) }
        fn create_pr_comment(&self, _pr_number: u64, _body: &str) -> Result<u64, AppError> { Ok(1) }
        fn update_pr_comment(&self, _comment_id: u64, _body: &str) -> Result<(), AppError> { Ok(()) }
        fn ensure_label(&self, _label: &str, _color: Option<&str>) -> Result<(), AppError> { Ok(()) }
        fn add_label_to_pr(&self, _pr_number: u64, _label: &str) -> Result<(), AppError> { Ok(()) }
        fn add_label_to_issue(&self, _issue_number: u64, _label: &str) -> Result<(), AppError> { Ok(()) }
        fn enable_automerge(&self, _pr_number: u64) -> Result<(), AppError> { Ok(()) }
        fn list_pr_files(&self, _pr_number: u64) -> Result<Vec<String>, AppError> { Ok(Vec::new()) }
    }

    fn make_config() -> MockConfig {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Implementer, "jules-implementer-".to_string());
        MockConfig {
            mock_tag: "mock-test-impl".to_string(),
            branch_prefixes: prefixes,
            default_branch: "main".to_string(),
            jules_branch: "jules".to_string(),
            issue_labels: vec!["bugs".to_string()],
        }
    }

    #[test]
    fn mock_implementer_creates_pr_for_valid_requirement() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        let req_path = PathBuf::from(".jules/exchange/requirements/req.yml");
        workspace.write_file(req_path.to_str().unwrap(), "id: abc123\nlabel: bugs\n").unwrap();

        let options = RunOptions {
            layer: Layer::Implementer,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: Some(req_path.clone()),
            mock: true,
            phase: None,
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.mock_branch.starts_with("jules-implementer-bugs-abc123-"));
        assert_eq!(output.mock_pr_number, 202);
    }

    #[test]
    fn mock_implementer_fails_if_label_not_allowed() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config(); // Allows "bugs"

        let req_path = PathBuf::from(".jules/exchange/requirements/req.yml");
        workspace.write_file(req_path.to_str().unwrap(), "id: abc123\nlabel: features\n").unwrap(); // "features" not allowed

        let options = RunOptions {
            layer: Layer::Implementer,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: Some(req_path),
            mock: true,
            phase: None,
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::Validation(msg)) if msg.contains("not defined in github-labels.json")));
    }
}
