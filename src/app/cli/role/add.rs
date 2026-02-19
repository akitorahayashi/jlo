use super::layer_selection::{parse_multi_role_layer, prompt_multi_role_layer};

use crate::domain::{AppError, BuiltinRoleEntry, Layer};
use dialoguer::Select;
use std::collections::BTreeMap;

const BACK_OPTION_LABEL: &str = "[back]";

pub fn run(layer: Option<String>, roles: Vec<String>) -> Result<(), AppError> {
    let Some((layer, roles)) = resolve_inputs(layer, roles)? else {
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

fn resolve_inputs(
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

enum BuiltinRoleSelection {
    Selected(String),
    BackToLayer,
    Cancel,
}

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
            category_items.push(BACK_OPTION_LABEL.to_string());
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
        role_items.push(BACK_OPTION_LABEL.to_string());

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
