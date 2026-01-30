use std::io::{BufRead, IsTerminal};

use dialoguer::Select;

use crate::app::AppContext;
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

/// Execute the template command.
///
/// Creates a new role directory under the specified layer with
/// pre-filled role.yml and prompt.yml based on the layer archetype.
pub fn execute<W, R, C>(
    ctx: &AppContext<W, R, C>,
    layer_arg: Option<&str>,
    role_name_arg: Option<&str>,
    workstream_arg: Option<&str>,
) -> Result<String, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    if !ctx.workspace().exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Resolve layer
    let layer = match layer_arg {
        Some(name) => {
            Layer::from_dir_name(name).ok_or_else(|| AppError::InvalidLayer(name.to_string()))?
        }
        None => select_layer()?,
    };

    // Reject single-role layers (Planners, Implementers)
    if layer.is_single_role() {
        return Err(AppError::SingleRoleLayerTemplate(layer.dir_name().to_string()));
    }

    // Get role name
    let role_name = match role_name_arg {
        Some(name) => name.to_string(),
        None => prompt_role_name()?,
    };

    // Validate role name
    let role_id = RoleId::new(&role_name)?;

    // Check if role already exists
    if ctx.workspace().role_exists_in_layer(layer, &role_id) {
        return Err(AppError::RoleExists { role: role_name, layer: layer.dir_name().to_string() });
    }

    // Resolve workstream for observers/deciders
    let workstream = match (layer, workstream_arg) {
        (Layer::Observers | Layer::Deciders, Some(ws)) => {
            // Validate workstream exists
            if !ctx.workspace().workstream_exists(ws) {
                return Err(AppError::ConfigError(format!(
                    "Workstream '{}' does not exist. Create it with: jlo workstream new {}",
                    ws, ws
                )));
            }
            Some(ws.to_string())
        }
        (Layer::Observers | Layer::Deciders, None) => {
            // Validate default workstream exists
            if !ctx.workspace().workstream_exists("generic") {
                return Err(AppError::ConfigError(
                    "Default workstream 'generic' does not exist. Run 'jlo init' or create it with: jlo workstream new generic".to_string()
                ));
            }
            Some("generic".to_string())
        }
        _ => None,
    };

    // Generate role.yml and prompt.yml content
    let role_yaml = ctx.templates().generate_role_yaml(&role_name, layer);
    let mut prompt_yaml = ctx.templates().generate_prompt_yaml_template(&role_name, layer);

    // Replace ROLE_NAME placeholder
    prompt_yaml = prompt_yaml.replace("ROLE_NAME", &role_name);

    // Replace workstream placeholder if applicable
    if let Some(ws) = &workstream {
        prompt_yaml = prompt_yaml.replace("workstream: generic", &format!("workstream: {}", ws));
    }

    // Determine if this layer type gets notes/
    let has_notes = matches!(layer, Layer::Observers);

    // Scaffold the role
    ctx.workspace().scaffold_role_in_layer(
        layer,
        &role_id,
        &role_yaml,
        Some(&prompt_yaml),
        has_notes,
    )?;

    Ok(format!("{}/{}", layer.dir_name(), role_name))
}

/// Interactive layer selection.
///
/// Only shows multi-role layers (Observers, Deciders) since single-role
/// layers (Planners, Implementers) do not support custom templates.
fn select_layer() -> Result<Layer, AppError> {
    // Filter to only multi-role layers
    let multi_role_layers: Vec<Layer> =
        Layer::ALL.iter().filter(|l| !l.is_single_role()).copied().collect();

    let items: Vec<String> = multi_role_layers
        .iter()
        .map(|l| format!("{} - {}", l.display_name(), l.description()))
        .collect();

    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        let selection = Select::new()
            .with_prompt("Select a layer")
            .items(&items)
            .default(0)
            .interact()
            .map_err(|e| AppError::config_error(format!("Layer selection failed: {}", e)))?;

        Ok(multi_role_layers[selection])
    } else {
        // Non-interactive: read from stdin
        let mut input = String::new();
        let mut stdin = std::io::stdin().lock();
        stdin
            .read_line(&mut input)
            .map_err(|e| AppError::config_error(format!("Failed to read layer: {}", e)))?;

        let trimmed = input.trim();

        // Try as 1-based index (only for multi-role layers)
        if let Ok(index) = trimmed.parse::<usize>()
            && index >= 1
            && index <= multi_role_layers.len()
        {
            return Ok(multi_role_layers[index - 1]);
        }

        // Try as layer name (validate it's a multi-role layer)
        let layer = Layer::from_dir_name(trimmed)
            .ok_or_else(|| AppError::InvalidLayer(trimmed.to_string()))?;

        if layer.is_single_role() {
            return Err(AppError::SingleRoleLayerTemplate(layer.dir_name().to_string()));
        }

        Ok(layer)
    }
}

/// Prompt for role name interactively.
fn prompt_role_name() -> Result<String, AppError> {
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        print!("Enter role name: ");
        use std::io::Write;
        std::io::stdout().flush()?;
    }

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| AppError::config_error(format!("Failed to read role name: {}", e)))?;

    let name = input.trim().to_string();
    if name.is_empty() {
        return Err(AppError::config_error("Role name cannot be empty"));
    }

    Ok(name)
}
