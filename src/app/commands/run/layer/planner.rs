use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunRuntimeOptions;
use crate::app::commands::run::input::{detect_repository_source, load_mock_config};
use crate::domain::layers::execute::starting_branch::resolve_starting_branch;
use crate::domain::layers::execute::validate_requirement_path;
use crate::domain::prompt_assemble::{PromptAssetLoader, PromptContext, assemble_prompt};
use crate::domain::{AppError, ControlPlaneConfig, Layer, MockConfig, MockOutput, RunOptions};
use crate::ports::{
    AutomationMode, Git, GitHub, JloStore, JulesClient, JulesStore, RepositoryFilesystem,
    SessionRequest,
};

use super::super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct PlannerLayer;

impl<W> LayerStrategy<W> for PlannerLayer
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
        target: &RunOptions,
        runtime: &RunRuntimeOptions,
        config: &ControlPlaneConfig,
        git: &dyn Git,
        github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if runtime.mock {
            let mock_config = load_mock_config(jules_path, repository)?;
            let output = execute_mock(jules_path, target, &mock_config, git, github, repository)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::super::mock::mock_execution::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::super::mock::mock_execution::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec!["planner".to_string()],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            runtime.prompt_preview,
            runtime.branch.as_deref(),
            target.requirement.as_deref(),
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
    let requirement_path = requirement_path.ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for planner".to_string())
    })?;
    let requirement_info = validate_requirement_path(requirement_path, repository)?;
    let requirement_content = repository.read_file(&requirement_info.requirement_path_str)?;

    let starting_branch = resolve_starting_branch(Layer::Planner, config, branch);

    if prompt_preview {
        execute_prompt_preview(
            jules_path,
            &starting_branch,
            &requirement_content,
            requirement_path,
            repository,
        )?;
        return Ok(RunResult {
            roles: vec!["planner".to_string()],
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
        requirement_path,
        repository,
    )?;

    Ok(RunResult {
        roles: vec!["planner".to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
        cleanup_requirement: None,
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
    requirement_path: &Path,
    repository: &W,
) -> Result<String, AppError> {
    println!("Executing {}...", Layer::Planner.display_name());

    let mut prompt = assemble_planner_prompt(jules_path, repository)?;

    prompt.push_str("\n---\n# Requirement Content\n");
    prompt.push_str(&format!("File: {}\n\n", requirement_path.display()));
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

fn assemble_planner_prompt<
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
    repository: &W,
) -> Result<String, AppError> {
    let (prompt, seed_ops) = assemble_prompt(
        jules_path,
        Layer::Planner,
        &PromptContext::new(),
        repository,
        crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset,
    )
    .map_err(|e| AppError::InternalError(e.to_string()))?;
    super::execute_seed_ops(seed_ops, repository)?;
    Ok(prompt.content)
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
    requirement_path: &Path,
    repository: &W,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", Layer::Planner.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Requirement content: {} chars\n", requirement_content.len());

    println!("Prompt template: planner/planner_prompt.j2 (embedded)");
    println!("Contracts: planner/contracts.yml (embedded)");

    if let Ok(mut prompt) = assemble_planner_prompt(jules_path, repository) {
        prompt.push_str("\n---\n# Requirement Content\n");
        prompt.push_str(&format!("File: {}\n\n", requirement_path.display()));
        prompt.push_str(requirement_content);

        println!(
            "Assembled prompt: {} chars (Prompt + Requirement Path + Requirement Content)",
            prompt.len()
        );
    }

    println!("\nWould execute 1 session");
    Ok(())
}

fn execute_mock<G, H, W>(
    _jules_path: &Path,
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
    let requirement_path = options.requirement.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for planner".to_string())
    })?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Planner, &timestamp)?;

    println!("Mock planner: creating branch {}", branch_name);

    // Planner mock always operates on the worker branch contract (`jules_worker_branch`).
    // Runtime branch overrides apply to real-mode dispatch only.
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_worker_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Read and modify requirement file
    let requirement_path_str = requirement_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid requirement path".to_string()))?;

    let requirement_content = repository.read_file(requirement_path_str)?;

    // Update requirement: preserve decider reason and set implementation_ready to true
    let updated_content = requirement_content
        .replace("implementation_ready: false", "implementation_ready: true")
        + &format!(
            r#"
# Mock planner expansion
expanded_at: "{}"
expanded_by: mock-planner
analysis_details: |
  Mock deep analysis performed by jlo --mock for workflow validation.
  Mock tag: {}

  ## Impact Analysis
  - Mock impact area 1
  - Mock impact area 2

  ## Implementation Notes
  - No actual analysis performed (mock mode)
"#,
            Utc::now().to_rfc3339(),
            config.mock_tag
        );

    repository.write_file(requirement_path_str, &updated_content)?;

    // Commit and push
    let files: Vec<&Path> = vec![requirement_path.as_path()];
    git.commit_files(&format!("[{}] planner: analysis complete", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_worker_branch,
        &format!("[{}] Planner analysis", config.mock_tag),
        &format!(
            "Mock planner run for workflow validation.\n\nMock tag: `{}`\nRequirement: `{}`",
            config.mock_tag,
            requirement_path.display()
        ),
    )?;

    println!("Mock planner: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}
