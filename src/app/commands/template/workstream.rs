use std::io::IsTerminal;

use dialoguer::{Input, Select};

use crate::app::AppContext;
use crate::domain::{AppError, Layer};
use crate::ports::{RoleTemplatePort, WorkspacePort};

pub(super) fn resolve_workstream<W, R>(
    ctx: &AppContext<W, R>,
    layer: Layer,
    workstream_arg: Option<&str>,
) -> Result<Option<String>, AppError>
where
    W: WorkspacePort,
    R: RoleTemplatePort,
{
    if !matches!(layer, Layer::Observers | Layer::Deciders) {
        return Ok(None);
    }

    match workstream_arg {
        Some(ws) => {
            if !ctx.workspace().workstream_exists(ws) {
                return Err(AppError::Validation(format!(
                    "Workstream '{}' does not exist. Run 'jlo template' and select Workstream to create it.",
                    ws
                )));
            }
            Ok(Some(ws.to_string()))
        }
        None => {
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                let selected = select_workstream(ctx)?;
                Ok(Some(selected))
            } else {
                Err(AppError::MissingArgument(
                    "Workstream is required for observers and deciders when running non-interactively. Use --workstream or run without args to use the wizard.".into(),
                ))
            }
        }
    }
}

fn select_workstream<W, R>(ctx: &AppContext<W, R>) -> Result<String, AppError>
where
    W: WorkspacePort,
    R: RoleTemplatePort,
{
    let workstreams = ctx.workspace().list_workstreams()?;
    if workstreams.is_empty() {
        return create_workstream(ctx);
    }

    let mut items = workstreams.clone();
    items.push("Create new workstream".to_string());

    let selection = Select::new()
        .with_prompt("Select target workstream")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| AppError::InternalError(format!("Workstream selection failed: {e}")))?;

    if selection == items.len() - 1 {
        create_workstream(ctx)
    } else {
        Ok(workstreams[selection].clone())
    }
}

pub(super) fn create_workstream<W, R>(ctx: &AppContext<W, R>) -> Result<String, AppError>
where
    W: WorkspacePort,
    R: RoleTemplatePort,
{
    let name = prompt_workstream_name()?;
    ctx.workspace().create_workstream(&name)?;
    Ok(name)
}

fn prompt_workstream_name() -> Result<String, AppError> {
    let name: String = Input::new()
        .with_prompt("Enter new workstream name")
        .interact_text()
        .map_err(|e| AppError::InternalError(format!("Failed to read workstream name: {e}")))?;

    validate_workstream_name(&name).map(|_| name)
}

fn validate_workstream_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Workstream name cannot be empty.".into()));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(AppError::Validation(
            "Workstream name must be a single directory name.".into(),
        ));
    }

    Ok(())
}
