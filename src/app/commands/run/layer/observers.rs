use std::path::Path;

use crate::adapters::jules_client_http::HttpJulesClient;
use crate::domain::prompt_assembly::observers as observers_asm;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::WorkspaceStore;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::RunResult;
use crate::app::commands::run::config::{detect_repository_source, load_config};
use crate::app::commands::run::multi_role_execution::{
    dispatch_session, print_role_preview, validate_role_exists,
};

/// Execute a single observer role.
pub(crate) fn execute<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
) -> Result<RunResult, AppError> {
    let role = options
        .role
        .as_ref()
        .ok_or_else(|| AppError::MissingArgument("Role is required for observers".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Observers, role_id.as_str())?;

    let config = load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    let bridge_task = resolve_observer_bridge_task(jules_path, workspace)?;
    let input =
        observers_asm::ObserverPromptInput { role: role_id.as_str(), bridge_task: &bridge_task };

    if options.prompt_preview {
        print_role_preview(jules_path, Layer::Observers, &role_id, &starting_branch);
        let assembled = observers_asm::assemble(jules_path, &input, workspace)?;
        println!("  Assembled prompt: {} chars", assembled.content.len());
        println!("\nWould execute 1 session");
        return Ok(RunResult { roles: vec![role.clone()], prompt_preview: true, sessions: vec![] });
    }

    let source = detect_repository_source()?;
    let assembled = observers_asm::assemble(jules_path, &input, workspace)?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = dispatch_session(
        Layer::Observers,
        &role_id,
        assembled.content,
        &source,
        &starting_branch,
        &client,
    )?;

    Ok(RunResult { roles: vec![role.clone()], prompt_preview: false, sessions: vec![session_id] })
}

/// Read bridge_comments.yml content if any innovator persona has an idea.yml.
/// Returns empty string if no ideas exist. Fails if ideas exist but bridge task file is missing.
fn resolve_observer_bridge_task<W: WorkspaceStore>(
    jules_path: &Path,
    workspace: &W,
) -> Result<String, AppError> {
    let innovators = jules::innovators_dir(jules_path);
    let innovators_str = innovators.to_string_lossy();

    let has_ideas = workspace
        .list_dir(&innovators_str)
        .ok()
        .map(|entries| {
            entries.iter().any(|entry| {
                workspace.is_dir(&entry.to_string_lossy())
                    && workspace.file_exists(&entry.join("idea.yml").to_string_lossy())
            })
        })
        .unwrap_or(false);

    if !has_ideas {
        return Ok(String::new());
    }

    let bridge_path = jules::tasks_dir(jules_path, Layer::Observers).join("bridge_comments.yml");
    workspace.read_file(&bridge_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "Innovator ideas exist, but observer bridge task file is missing: expected {}",
            bridge_path.display()
        ))
    })
}
