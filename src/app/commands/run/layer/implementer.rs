use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Deserialize;

use super::super::mock::mock_execution::MockExecutionService;
use crate::app::commands::run::input::{detect_repository_source, load_mock_config};
use crate::domain::layers::prompt_assembly::{
    AssembledPrompt, PromptAssetLoader, PromptContext, assemble_prompt,
};
use crate::domain::{AppError, Layer, MockConfig, MockOutput, RunConfig, RunOptions};
use crate::ports::{
    AutomationMode, Git, GitHub, JloStore, JulesClient, JulesStore, RepositoryFilesystem,
    SessionRequest,
};

use super::super::requirement_path::validate_requirement_path;
use super::super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct ImplementerLayer;

impl<W> LayerStrategy<W> for ImplementerLayer
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
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn Git,
        github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if options.mock {
            let mock_config = load_mock_config(jules_path, options, repository)?;
            let _output = execute_mock(jules_path, options, &mock_config, git, github, repository)?;
            let cleanup_requirement = options.requirement.clone();
            // Mock output is written by execute_mock's service.finish()
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
            repository,
            client_factory,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    requirement_path: Option<&Path>,
    config: &RunConfig,
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
    let requirement_path = requirement_path.ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for implementer".to_string())
    })?;
    let requirement_info = validate_requirement_path(requirement_path, repository)?;

    let requirement_content = repository.read_file(&requirement_info.requirement_path_str)?;

    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jlo_target_branch.clone());

    if prompt_preview {
        execute_prompt_preview(jules_path, &starting_branch, &requirement_content, repository)?;
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
        repository,
    )?;

    // Return cleanup requirement path so caller can clean it up
    Ok(RunResult {
        roles: vec!["implementer".to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
        cleanup_requirement: Some(PathBuf::from(requirement_info.requirement_path_str)),
    })
}

fn execute_session<
    C: JulesClient + ?Sized,
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
    starting_branch: &str,
    source: &str,
    client: &C,
    requirement_content: &str,
    repository: &W,
) -> Result<String, AppError> {
    println!("Executing {}...", Layer::Implementer.display_name());

    let mut prompt = assemble_implementer_prompt(jules_path, requirement_content, repository)?;

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

fn assemble_implementer_prompt<
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
    requirement_content: &str,
    repository: &W,
) -> Result<String, AppError> {
    let label = extract_requirement_label(requirement_content)?;
    let task_content = resolve_implementer_task(jules_path, &label, repository)?;

    let context = PromptContext::new().with_var("task", task_content);

    assemble_prompt(jules_path, Layer::Implementer, &context, repository)
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

    if !crate::domain::roles::validation::validate_safe_path_component(label) {
        return Err(AppError::Validation(format!(
            "Invalid label '{}': must be a safe path component",
            label
        )));
    }

    Ok(label.to_string())
}

fn resolve_implementer_task<W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader>(
    jules_path: &Path,
    label: &str,
    repository: &W,
) -> Result<String, AppError> {
    let task_path = crate::domain::layers::paths::tasks_dir(jules_path, Layer::Implementer)
        .join(format!("{}.yml", label));

    repository.read_file(&task_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "No task file for label '{}': expected {}",
            label,
            task_path.display()
        ))
    })
}

fn execute_prompt_preview<
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
    starting_branch: &str,
    requirement_content: &str,
    repository: &W,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", Layer::Implementer.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Requirement content: {} chars\n", requirement_content.len());

    let prompt_path = crate::domain::layers::paths::prompt_template(jules_path, Layer::Implementer);
    let contracts_path = crate::domain::layers::paths::contracts(jules_path, Layer::Implementer);

    println!("Prompt: {}", prompt_path.display());
    if contracts_path.exists() {
        println!("Contracts: {}", contracts_path.display());
    }

    let mut prompt = assemble_implementer_prompt(jules_path, requirement_content, repository)?;
    prompt.push_str("\n---\n# Requirement Content\n");
    prompt.push_str(requirement_content);

    println!("Assembled prompt: {} chars (Prompt + No Path + Requirement Content)", prompt.len());

    println!("\nWould execute 1 session");
    Ok(())
}

fn execute_mock<G, H, W>(
    jules_path: &Path,
    options: &RunOptions,
    config: &MockConfig,
    git: &G,
    github: &H,
    repository: &W,
) -> Result<MockOutput, AppError>
where
    G: Git + ?Sized,
    H: GitHub + ?Sized,
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    let service = MockExecutionService::new(jules_path, config, git, github, repository);

    let original_branch = git.get_current_branch()?;

    let requirement_path = options.requirement.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for implementer".to_string())
    })?;

    // Parse requirement to get label and id
    let requirement_path_str = requirement_path
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Invalid requirement path".to_string()))?;

    let requirement_content = repository.read_file(requirement_path_str)?;
    let (label, issue_id) = parse_requirement_for_branch(&requirement_content, requirement_path)?;
    if !config.issue_labels.contains(&label) {
        return Err(AppError::InvalidConfig(format!(
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
    let base_branch = options.branch.as_deref().unwrap_or(&config.jlo_target_branch);
    service.fetch_and_checkout_base(base_branch)?;
    service.checkout_new_branch(&branch_name)?;

    // Create minimal mock file to have a commit
    let mock_file_path = format!(".{}", config.mock_tag);
    let mock_content = format!(
        "# Mock implementation marker\n# Mock tag: {}\n# Issue: {}\n# Created: {}\n",
        config.mock_tag,
        issue_id,
        Utc::now().to_rfc3339()
    );

    repository.write_file(&mock_file_path, &mock_content)?;

    // Commit and push
    let mock_path = Path::new(&mock_file_path);
    let files: Vec<&Path> = vec![mock_path];
    service.commit_and_push(
        &format!("[{}] implementer: mock implementation", config.mock_tag),
        &files,
        &branch_name,
    )?;

    // Create PR targeting default branch (NOT jules)
    let pr = service.create_pr(
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
        config.jules_worker_branch.as_str()
    } else {
        &original_branch
    };
    service.git.checkout_branch(restore_branch, false)?;

    let output = MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    };

    service.finish(&output)?;

    Ok(output)
}

fn parse_requirement_for_branch(content: &str, path: &Path) -> Result<(String, String), AppError> {
    #[derive(Deserialize)]
    struct RequirementMeta {
        label: Option<String>,
        id: Option<String>,
    }

    let parsed: RequirementMeta = serde_yaml::from_str(content).map_err(|err| {
        AppError::InvalidConfig(format!(
            "Requirement file must be valid YAML ({}): {}",
            path.display(),
            err
        ))
    })?;

    let label = parsed.label.filter(|value| !value.trim().is_empty()).ok_or_else(|| {
        AppError::InvalidConfig(format!("Requirement file missing label field: {}", path.display()))
    })?;
    if !crate::domain::roles::validation::validate_safe_path_component(&label) {
        return Err(AppError::InvalidConfig(format!(
            "Requirement label '{}' is not a safe path component: {}",
            label,
            path.display()
        )));
    }

    let id = parsed.id.filter(|value| !value.trim().is_empty()).ok_or_else(|| {
        AppError::InvalidConfig(format!("Requirement file missing id field: {}", path.display()))
    })?;

    if id.len() != 6 || !id.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit()) {
        return Err(AppError::InvalidConfig(format!(
            "Issue id must be 6 lowercase alphanumeric chars: {}",
            path.display()
        )));
    }

    Ok((label, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RepositoryFilesystem;
    use crate::testing::{FakeGit, FakeGitHub, TestStore};
    use std::collections::HashMap;

    fn make_config() -> MockConfig {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Implementer, "jules-implementer-".to_string());
        MockConfig {
            mock_tag: "mock-test-impl".to_string(),
            branch_prefixes: prefixes,
            jlo_target_branch: "main".to_string(),
            jules_worker_branch: "jules".to_string(),
            issue_labels: vec!["bugs".to_string()],
        }
    }

    #[test]
    fn mock_implementer_creates_pr_for_valid_requirement() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let req_path = PathBuf::from(".jules/exchange/requirements/req.yml");
        repository.write_file(req_path.to_str().unwrap(), "id: abc123\nlabel: bugs\n").unwrap();

        let options = RunOptions {
            layer: Layer::Implementer,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: Some(req_path.clone()),
            mock: true,
            task: None,
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &repository);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.mock_branch.starts_with("jules-implementer-bugs-abc123-"));
        assert_eq!(output.mock_pr_number, 101);
    }

    #[test]
    fn mock_implementer_fails_if_label_not_allowed() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config(); // Allows "bugs"

        let req_path = PathBuf::from(".jules/exchange/requirements/req.yml");
        repository.write_file(req_path.to_str().unwrap(), "id: abc123\nlabel: features\n").unwrap(); // "features" not allowed

        let options = RunOptions {
            layer: Layer::Implementer,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: Some(req_path),
            mock: true,
            task: None,
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &repository);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(AppError::InvalidConfig(msg)) if msg.contains("not defined in github-labels.json"))
        );
    }
}
