use std::io::IsTerminal;

use dialoguer::{Input, Select};

use crate::app::AppContext;
use crate::domain::{AppError, Layer};
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

pub(super) fn resolve_workstream<W, R, C>(
    ctx: &AppContext<W, R, C>,
    layer: Layer,
    workstream_arg: Option<&str>,
) -> Result<Option<String>, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    if !matches!(layer, Layer::Observers | Layer::Deciders) {
        return Ok(None);
    }

    match workstream_arg {
        Some(ws) => {
            if !ctx.workspace().workstream_exists(ws) {
                return Err(AppError::config_error(format!(
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
                Err(AppError::config_error(
                    "Workstream is required for observers and deciders when running non-interactively. Use --workstream or run without args to use the wizard.",
                ))
            }
        }
    }
}

fn select_workstream<W, R, C>(ctx: &AppContext<W, R, C>) -> Result<String, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
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
        .map_err(|e| AppError::config_error(format!("Workstream selection failed: {e}")))?;

    if selection == items.len() - 1 {
        create_workstream(ctx)
    } else {
        Ok(workstreams[selection].clone())
    }
}

pub(super) fn create_workstream<W, R, C>(ctx: &AppContext<W, R, C>) -> Result<String, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    let name = prompt_workstream_name()?;
    ctx.workspace().create_workstream(&name)?;
    Ok(name)
}

fn prompt_workstream_name() -> Result<String, AppError> {
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        let name: String = Input::new()
            .with_prompt("Enter new workstream name")
            .interact_text()
            .map_err(|e| AppError::config_error(format!("Failed to read workstream name: {e}")))?;

        return validate_workstream_name(&name).map(|_| name);
    }

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| AppError::config_error(format!("Failed to read workstream name: {e}")))?;

    let name = input.trim().to_string();
    validate_workstream_name(&name)?;
    Ok(name)
}

fn validate_workstream_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::config_error("Workstream name cannot be empty."));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(AppError::config_error("Workstream name must be a single directory name."));
    }

    Ok(())
}
