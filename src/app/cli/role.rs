//! Role command implementation.

use crate::app::api::ExistingRoleEntry;
use crate::domain::{AppError, BuiltinRoleEntry, Layer};
use clap::Subcommand;
use dialoguer::{Error as DialoguerError, Input, Select};
use std::collections::BTreeMap;
use std::io::ErrorKind;

#[derive(Subcommand)]
pub enum RoleCommands {
    /// Add a built-in role under .jlo/
    #[clap(visible_aliases = ["a", "ad"])]
    Add {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Built-in role name(s)
        roles: Vec<String>,
    },
    /// Create a new role under .jlo/
    #[clap(visible_aliases = ["c", "cr"])]
    Create {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Name for the new role
        role: Option<String>,
    },
    /// Delete a role from .jlo/
    #[clap(visible_aliases = ["d", "dl"])]
    Delete {
        /// Layer (observers, innovators)
        layer: Option<String>,
        /// Role name to delete
        role: Option<String>,
    },
}

pub fn run_role(command: RoleCommands) -> Result<(), AppError> {
    match command {
        RoleCommands::Add { layer, roles } => run_add(layer, roles),
        RoleCommands::Create { layer, role } => run_create(layer, role),
        RoleCommands::Delete { layer, role } => run_delete(layer, role),
    }
}

fn run_create(layer: Option<String>, role: Option<String>) -> Result<(), AppError> {
    let Some((layer, role)) = resolve_create_inputs(layer, role)? else {
        return Ok(());
    };
    let outcome = crate::app::api::role_create(&layer, &role)?;
    println!("✅ Created new {} at {}/", outcome.entity_type(), outcome.display_path());
    Ok(())
}

fn run_add(layer: Option<String>, roles: Vec<String>) -> Result<(), AppError> {
    let Some((layer, roles)) = resolve_add_inputs(layer, roles)? else {
        return Ok(());
    };

    for role in roles {
        let outcome = crate::app::api::role_add(&layer, &role)?;
        println!(
            "✅ Added {} '{}' in layer '{}' to {}",
            outcome.entity_type(),
            role,
            layer,
            outcome.display_path()
        );
    }
    Ok(())
}

fn run_delete(layer: Option<String>, role: Option<String>) -> Result<(), AppError> {
    let Some((layer, role)) = resolve_delete_inputs(layer, role)? else {
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

fn resolve_create_inputs(
    layer: Option<String>,
    role: Option<String>,
) -> Result<Option<(String, String)>, AppError> {
    let layer_enum = match layer {
        Some(value) => parse_multi_role_layer(&value)?,
        None => match prompt_multi_role_layer()? {
            Some(value) => value,
            None => return Ok(None),
        },
    };

    let role_value = match role {
        Some(value) => value,
        None => match prompt_role_name()? {
            Some(value) => value,
            None => return Ok(None),
        },
    };

    Ok(Some((layer_enum.dir_name().to_string(), role_value)))
}

fn resolve_add_inputs(
    layer: Option<String>,
    roles: Vec<String>,
) -> Result<Option<(String, Vec<String>)>, AppError> {
    if !roles.is_empty() {
        let layer_enum = match layer {
            Some(value) => parse_multi_role_layer(&value)?,
            None => match prompt_multi_role_layer()? {
                Some(value) => value,
                None => return Ok(None),
            },
        };
        return Ok(Some((layer_enum.dir_name().to_string(), roles)));
    }

    let catalog = crate::app::api::builtin_role_catalog()?;

    if let Some(value) = layer {
        let layer_enum = parse_multi_role_layer(&value)?;
        return match prompt_builtin_role(&catalog, layer_enum, false)? {
            BuiltinRoleSelection::Selected(role) => {
                Ok(Some((layer_enum.dir_name().to_string(), vec![role])))
            }
            BuiltinRoleSelection::Cancel => Ok(None),
            BuiltinRoleSelection::BackToLayer => unreachable!("layer is fixed"),
        };
    }

    loop {
        let Some(layer_enum) = prompt_multi_role_layer()? else {
            return Ok(None);
        };
        match prompt_builtin_role(&catalog, layer_enum, true)? {
            BuiltinRoleSelection::Selected(role) => {
                return Ok(Some((layer_enum.dir_name().to_string(), vec![role])));
            }
            BuiltinRoleSelection::BackToLayer => continue,
            BuiltinRoleSelection::Cancel => return Ok(None),
        }
    }
}

fn resolve_delete_inputs(
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

fn parse_multi_role_layer(value: &str) -> Result<Layer, AppError> {
    let layer =
        Layer::from_dir_name(value).ok_or(AppError::InvalidLayer { name: value.to_string() })?;
    if layer.is_single_role() {
        return Err(AppError::SingleRoleLayerTemplate(layer.dir_name().to_string()));
    }
    Ok(layer)
}

fn prompt_multi_role_layer() -> Result<Option<Layer>, AppError> {
    let layers: Vec<Layer> =
        Layer::ALL.into_iter().filter(|layer| !layer.is_single_role()).collect();
    if layers.is_empty() {
        return Err(AppError::Validation("No multi-role layers available".to_string()));
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

enum BuiltinRoleSelection {
    Selected(String),
    BackToLayer,
    Cancel,
}

enum DeleteRoleSelection {
    Selected(String),
    BackToLayer,
    Cancel,
}

const MENU_BACK_OPTION: &str = "[back]";

fn prompt_builtin_role(
    catalog: &[BuiltinRoleEntry],
    layer: Layer,
    allow_layer_back: bool,
) -> Result<BuiltinRoleSelection, AppError> {
    let entries_by_category = catalog.iter().filter(|entry| entry.layer == layer).fold(
        BTreeMap::<&str, Vec<&BuiltinRoleEntry>>::new(),
        |mut map, entry| {
            map.entry(entry.category.as_str()).or_default().push(entry);
            map
        },
    );

    if entries_by_category.is_empty() {
        return Err(AppError::Validation(format!(
            "No builtin roles available for layer '{}'",
            layer.dir_name()
        )));
    }

    let categories: Vec<&str> = entries_by_category.keys().copied().collect();
    loop {
        let mut category_items: Vec<String> =
            categories.iter().map(|value| value.to_string()).collect();
        if allow_layer_back {
            category_items.push(MENU_BACK_OPTION.to_string());
        }

        let category_index = Select::new()
            .with_prompt("Select category")
            .items(&category_items)
            .default(0)
            .interact_opt()
            .map_err(|err| AppError::Validation(format!("Failed to select category: {}", err)))?;

        let Some(category_index) = category_index else {
            return Ok(BuiltinRoleSelection::Cancel);
        };

        if allow_layer_back && category_index == category_items.len() - 1 {
            return Ok(BuiltinRoleSelection::BackToLayer);
        }

        let selected_category = categories[category_index];
        let roles = entries_by_category.get(selected_category).expect("category exists");

        let mut role_items: Vec<String> = roles
            .iter()
            .map(|entry| format!("{}: {}", entry.name.as_str(), entry.summary))
            .collect();
        role_items.push(MENU_BACK_OPTION.to_string());

        let role_index = Select::new()
            .with_prompt("Select role")
            .items(&role_items)
            .default(0)
            .interact_opt()
            .map_err(|err| AppError::Validation(format!("Failed to select role: {}", err)))?;

        let Some(role_index) = role_index else {
            return Ok(BuiltinRoleSelection::Cancel);
        };

        if role_index == role_items.len() - 1 {
            continue;
        }

        return Ok(BuiltinRoleSelection::Selected(roles[role_index].name.as_str().to_string()));
    }
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
        items.push(MENU_BACK_OPTION.to_string());
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

fn prompt_role_name() -> Result<Option<String>, AppError> {
    match Input::new().with_prompt("Role name").interact_text() {
        Ok(value) => Ok(Some(value)),
        Err(DialoguerError::IO(err)) if err.kind() == ErrorKind::Interrupted => Ok(None),
        Err(err) => Err(AppError::Validation(format!("Failed to read role name: {}", err))),
    }
}
