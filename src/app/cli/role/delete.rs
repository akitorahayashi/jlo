use super::layer_selection::parse_multi_role_layer;

use crate::app::api::ExistingRoleEntry;
use crate::domain::{AppError, Layer};
use dialoguer::Select;

const BACK_OPTION_LABEL: &str = "[back]";

pub fn run(layer: Option<String>, role: Option<String>) -> Result<(), AppError> {
    let Some((layer, role)) = resolve_inputs(layer, role)? else {
        return Ok(());
    };

    let outcome = crate::app::api::role_delete(&layer, &role)?;
    println!(
        "✅ Deleted {} '{}' in layer '{}' from {} and unscheduled it in .jlo/config.toml",
        outcome.entity_type(),
        role,
        layer,
        outcome.display_path()
    );
    Ok(())
}

fn resolve_inputs(
    layer: Option<String>,
    role: Option<String>,
) -> Result<Option<(String, String)>, AppError> {
    if let (Some(layer_value), Some(role_value)) = (layer.as_deref(), role.as_deref()) {
        let layer_enum = parse_multi_role_layer(layer_value)?;
        return Ok(Some((layer_enum.dir_name().to_string(), role_value.to_string())));
    }

    let discovered = crate::app::api::discover_roles()?;
    if discovered.is_empty() {
        return Err(AppError::Validation(
            "No roles available to delete under .jlo/roles.".to_string(),
        ));
    }

    if let Some(layer_value) = layer {
        let layer_enum = parse_multi_role_layer(&layer_value)?;
        let role_value = match prompt_deletable_role(&discovered, layer_enum, false)? {
            DeleteRoleSelection::Selected(value) => value,
            DeleteRoleSelection::Cancel => return Ok(None),
            DeleteRoleSelection::BackToLayer => unreachable!("layer is fixed"),
        };
        return Ok(Some((layer_enum.dir_name().to_string(), role_value)));
    }

    loop {
        let Some(layer_enum) = prompt_deletable_layer(&discovered)? else {
            return Ok(None);
        };

        match prompt_deletable_role(&discovered, layer_enum, true)? {
            DeleteRoleSelection::Selected(role_name) => {
                return Ok(Some((layer_enum.dir_name().to_string(), role_name)));
            }
            DeleteRoleSelection::BackToLayer => continue,
            DeleteRoleSelection::Cancel => return Ok(None),
        }
    }
}

fn prompt_deletable_layer(discovered: &[ExistingRoleEntry]) -> Result<Option<Layer>, AppError> {
    let layers: Vec<Layer> = Layer::ALL
        .into_iter()
        .filter(|layer| !layer.is_single_role())
        .filter(|layer| discovered.iter().any(|entry| entry.layer == *layer))
        .collect();
    if layers.is_empty() {
        return Err(AppError::Validation(
            "No roles available to delete under .jlo/roles.".to_string(),
        ));
    }

    let items: Vec<String> =
        layers.iter().map(|layer| layer.display_name().to_lowercase()).collect();
    let selection = Select::new()
        .with_prompt("Select layer")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|err| AppError::Validation(format!("Failed to select layer: {}", err)))?;

    Ok(selection.map(|index| layers[index]))
}

enum DeleteRoleSelection {
    Selected(String),
    BackToLayer,
    Cancel,
}

fn prompt_deletable_role(
    discovered: &[ExistingRoleEntry],
    layer: Layer,
    allow_layer_back: bool,
) -> Result<DeleteRoleSelection, AppError> {
    let mut role_names: Vec<String> = discovered
        .iter()
        .filter(|entry| entry.layer == layer)
        .map(|entry| entry.role.clone())
        .collect();
    role_names.sort();
    role_names.dedup();

    if role_names.is_empty() {
        return Err(AppError::Validation(format!(
            "No roles available to delete in layer '{}'.",
            layer.dir_name()
        )));
    }

    let mut items = role_names.clone();
    if allow_layer_back {
        items.push(BACK_OPTION_LABEL.to_string());
    }

    let index = Select::new()
        .with_prompt("Select role")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|err| AppError::Validation(format!("Failed to select role: {}", err)))?;

    let Some(index) = index else {
        return Ok(DeleteRoleSelection::Cancel);
    };

    if allow_layer_back && index == items.len() - 1 {
        return Ok(DeleteRoleSelection::BackToLayer);
    }

    Ok(DeleteRoleSelection::Selected(role_names[index].clone()))
}
