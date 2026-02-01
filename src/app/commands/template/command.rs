use std::io::IsTerminal;

use dialoguer::Select;

use crate::app::AppContext;
use crate::domain::{AppError, Layer};
use crate::ports::{RoleTemplateStore, WorkspaceStore};
use crate::services::role_factory::RoleFactory;

use super::outcome::TemplateOutcome;
use super::wizard::run_template_wizard;
use super::workstream::resolve_workstream;

/// Execute the template command.
///
/// Creates a new role directory under the specified layer with
/// pre-filled role.yml and prompt.yml.
pub fn execute<W, R>(
    ctx: &AppContext<W, R>,
    layer_arg: Option<&str>,
    role_name_arg: Option<&str>,
    workstream_arg: Option<&str>,
) -> Result<TemplateOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    if !ctx.workspace().exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let should_use_wizard =
        layer_arg.is_none() && role_name_arg.is_none() && workstream_arg.is_none();
    if should_use_wizard {
        if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
            return Err(AppError::config_error(
                "Interactive template wizard requires a TTY. Provide --layer and --name (and --workstream for observer/decider roles).",
            ));
        }

        return run_template_wizard(ctx);
    }

    let layer = resolve_layer(layer_arg)?;

    create_role_from_template(ctx, layer, role_name_arg, workstream_arg)
}

pub(super) fn create_role_from_template<W, R>(
    ctx: &AppContext<W, R>,
    layer: Layer,
    role_name_arg: Option<&str>,
    workstream_arg: Option<&str>,
) -> Result<TemplateOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    if layer.is_single_role() {
        return Err(AppError::SingleRoleLayerTemplate(layer.dir_name().to_string()));
    }

    let role_name = resolve_role_name(role_name_arg)?;
    let workstream = resolve_workstream(ctx, layer, workstream_arg)?;

    RoleFactory::create_role(
        ctx.workspace(),
        ctx.templates(),
        layer,
        &role_name,
        workstream.as_deref(),
    )?;

    Ok(TemplateOutcome::Role { layer, role: role_name })
}

fn resolve_layer(layer_arg: Option<&str>) -> Result<Layer, AppError> {
    match layer_arg {
        Some(name) => {
            Ok(Layer::from_dir_name(name)
                .ok_or_else(|| AppError::InvalidLayer(name.to_string()))?)
        }
        None => {
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                Ok(select_layer()?)
            } else {
                Err(AppError::config_error("Layer is required when running non-interactively."))
            }
        }
    }
}

fn resolve_role_name(role_name_arg: Option<&str>) -> Result<String, AppError> {
    match role_name_arg {
        Some(name) => Ok(name.to_string()),
        None => {
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                prompt_role_name()
            } else {
                Err(AppError::config_error("Role name is required when running non-interactively."))
            }
        }
    }
}

/// Interactive layer selection.
///
/// Only shows multi-role layers (Observers, Deciders) since single-role
/// layers (Planners, Implementers) do not support custom templates.
fn select_layer() -> Result<Layer, AppError> {
    let multi_role_layers: Vec<Layer> =
        Layer::ALL.iter().filter(|layer| !layer.is_single_role()).copied().collect();

    let items: Vec<String> = multi_role_layers
        .iter()
        .map(|layer| format!("{} - {}", layer.display_name(), layer.description()))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a layer")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| AppError::config_error(format!("Layer selection failed: {e}")))?;

    Ok(multi_role_layers[selection])
}

/// Prompt for role name interactively.
fn prompt_role_name() -> Result<String, AppError> {
    print!("Enter role name: ");
    use std::io::Write;
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| AppError::config_error(format!("Failed to read role name: {e}")))?;

    let name = input.trim().to_string();
    if name.is_empty() {
        return Err(AppError::config_error("Role name cannot be empty"));
    }

    Ok(name)
}
