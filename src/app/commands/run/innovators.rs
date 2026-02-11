use std::path::Path;

use crate::adapters::jules_client_http::HttpJulesClient;
use crate::domain::prompt_assembly::innovators as innovators_asm;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::WorkspaceStore;

use super::RunOptions;
use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::multi_role_execution::{dispatch_session, print_role_preview, validate_role_exists};

/// Execute a single innovator role.
pub(crate) fn execute<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
) -> Result<RunResult, AppError> {
    let role = options
        .role
        .as_ref()
        .ok_or_else(|| AppError::MissingArgument("Role is required for innovators".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Innovators, role_id.as_str())?;

    let config = load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    let phase = options.phase.as_deref().ok_or_else(|| {
        AppError::MissingArgument(
            "--phase is required for innovators (creation or refinement)".to_string(),
        )
    })?;
    let task_content = resolve_innovator_task(jules_path, phase, workspace)?;
    let input =
        innovators_asm::InnovatorPromptInput { role: role_id.as_str(), phase, task: &task_content };

    if options.prompt_preview {
        print_role_preview(jules_path, Layer::Innovators, &role_id, &starting_branch);
        let assembled = innovators_asm::assemble(jules_path, &input, workspace)?;
        println!("  Assembled prompt: {} chars", assembled.content.len());
        println!("\nWould execute 1 session");
        return Ok(RunResult { roles: vec![role.clone()], prompt_preview: true, sessions: vec![] });
    }

    let source = detect_repository_source()?;
    let assembled = innovators_asm::assemble(jules_path, &input, workspace)?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = dispatch_session(
        Layer::Innovators,
        &role_id,
        assembled.content,
        &source,
        &starting_branch,
        &client,
    )?;

    Ok(RunResult { roles: vec![role.clone()], prompt_preview: false, sessions: vec![session_id] })
}

/// Read the task file content for the current innovator phase.
/// Maps phase to task filename: creation → create_idea.yml, refinement → refine_proposal.yml.
fn resolve_innovator_task<W: WorkspaceStore>(
    jules_path: &Path,
    phase: &str,
    workspace: &W,
) -> Result<String, AppError> {
    let filename = match phase {
        "creation" => "create_idea.yml",
        "refinement" => "refine_proposal.yml",
        _ => {
            return Err(AppError::Validation(format!(
                "Unknown innovator phase '{}': must be 'creation' or 'refinement'",
                phase
            )));
        }
    };
    let task_path = jules::tasks_dir(jules_path, Layer::Innovators).join(filename);
    workspace.read_file(&task_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "No task file for innovator phase '{}': expected {}",
            phase,
            task_path.display()
        ))
    })
}
