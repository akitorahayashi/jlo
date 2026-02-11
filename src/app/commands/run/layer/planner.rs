use std::path::Path;

use crate::adapters::jules_client_http::HttpJulesClient;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer};
use crate::ports::{AutomationMode, JulesClient, SessionRequest, WorkspaceStore};

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::RunResult;
use crate::app::commands::run::config::{detect_repository_source, load_config};
use crate::app::commands::run::prompt::assemble_single_role_prompt;
use crate::app::commands::run::requirement_execution::validate_requirement_path;

/// Execute the planner layer (single-role, issue-driven).
pub(crate) fn execute<W>(
    jules_path: &Path,
    options: &RunOptions,
    requirement_path: &Path,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let requirement_info = validate_requirement_path(requirement_path, workspace)?;
    let requirement_content = workspace.read_file(&requirement_info.requirement_path_str)?;
    let config = load_config(jules_path)?;

    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    if options.prompt_preview {
        execute_prompt_preview(
            jules_path,
            &starting_branch,
            &requirement_content,
            requirement_path,
            workspace,
        )?;
        return Ok(RunResult {
            roles: vec![Layer::Planner.dir_name().to_string()],
            prompt_preview: true,
            sessions: vec![],
        });
    }

    let source = detect_repository_source()?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = execute_session(
        jules_path,
        &starting_branch,
        &source,
        &client,
        &requirement_content,
        requirement_path,
        workspace,
    )?;

    Ok(RunResult {
        roles: vec![Layer::Planner.dir_name().to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_session<C: JulesClient, W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    starting_branch: &str,
    source: &str,
    client: &C,
    requirement_content: &str,
    requirement_path: &Path,
    workspace: &W,
) -> Result<String, AppError> {
    println!("Executing {}...", Layer::Planner.display_name());

    let mut prompt = assemble_planner_prompt(jules_path, workspace)?;

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

fn assemble_planner_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    workspace: &W,
) -> Result<String, AppError> {
    assemble_single_role_prompt(jules_path, Layer::Planner, workspace)
}

fn execute_prompt_preview<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    starting_branch: &str,
    requirement_content: &str,
    requirement_path: &Path,
    workspace: &W,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", Layer::Planner.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Requirement content: {} chars\n", requirement_content.len());

    let prompt_path = jules::prompt_template(jules_path, Layer::Planner);
    let contracts_path = jules::contracts(jules_path, Layer::Planner);

    println!("Prompt: {}", prompt_path.display());
    if contracts_path.exists() {
        println!("Contracts: {}", contracts_path.display());
    }

    if let Ok(mut prompt) = assemble_planner_prompt(jules_path, workspace) {
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
