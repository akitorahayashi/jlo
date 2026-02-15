use crate::domain::{AppError, Layer, RoleId};
use crate::ports::RepositoryFilesystem;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

pub fn ensure_role_scheduled<W: RepositoryFilesystem>(
    repository: &W,
    layer: Layer,
    role: &RoleId,
) -> Result<bool, AppError> {
    if layer.is_single_role() {
        return Err(AppError::Validation(format!(
            "Layer '{}' does not support scheduling",
            layer.dir_name()
        )));
    }

    let config_path = ".jlo/config.toml";
    let content = repository.read_file(config_path)?;
    let mut doc = content.parse::<DocumentMut>().map_err(|err| {
        AppError::Validation(format!("Failed to parse .jlo/config.toml: {}", err))
    })?;

    let roles = layer_roles_mut(&mut doc, layer.dir_name())?;
    if contains_role(roles, role)? {
        return Ok(false);
    }

    roles.push(Value::InlineTable(scheduled_role_entry(role)));
    repository.write_file(config_path, &doc.to_string())?;
    Ok(true)
}

fn layer_roles_mut<'a>(
    doc: &'a mut DocumentMut,
    layer_name: &str,
) -> Result<&'a mut Array, AppError> {
    let layer_table =
        doc.entry(layer_name).or_insert(Item::Table(Table::new())).as_table_mut().ok_or_else(
            || {
                AppError::Validation(format!(
                    "Expected [{}] to be a table in .jlo/config.toml",
                    layer_name
                ))
            },
        )?;

    let roles_item = layer_table.entry("roles").or_insert(Item::Value(Value::Array(Array::new())));

    roles_item
        .as_value_mut()
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| AppError::Validation(format!("{}.roles must be an array", layer_name)))
}

fn contains_role(roles: &Array, role: &RoleId) -> Result<bool, AppError> {
    for entry in roles.iter() {
        let table = entry.as_inline_table().ok_or_else(|| {
            AppError::Validation(
                "Schedule role entry must be an inline table: { name = \"...\", enabled = ... }"
                    .to_string(),
            )
        })?;
        let Some(name) = table.get("name").and_then(|v| v.as_str()) else {
            return Err(AppError::Validation(
                "Schedule role entry is missing string field 'name'".to_string(),
            ));
        };
        if name == role.as_str() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn scheduled_role_entry(role: &RoleId) -> InlineTable {
    let mut entry = InlineTable::new();
    entry.insert("name", Value::from(role.as_str()));
    entry.insert("enabled", Value::from(true));
    entry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RepositoryFilesystem;
    use crate::testing::TestStore;

    fn role_names(content: &str, layer: &str) -> Vec<String> {
        let value: toml::Value = toml::from_str(content).expect("config should parse");
        value
            .get(layer)
            .and_then(|layer_value| layer_value.get("roles"))
            .and_then(|roles| roles.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|role_value| {
                role_value.get("name").and_then(|name| name.as_str()).map(|name| name.to_string())
            })
            .collect()
    }

    #[test]
    fn ensure_role_scheduled_updates_observer_roster_in_config() {
        let repository = TestStore::new().with_file(
            ".jlo/config.toml",
            r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[observers]
roles = [
  { name = "consistency", enabled = true },
]
"#,
        );

        let updated = ensure_role_scheduled(
            &repository,
            Layer::Observers,
            &RoleId::new("librarian").expect("valid role id"),
        )
        .expect("schedule update should succeed");
        assert!(updated);

        let actual = repository.read_file(".jlo/config.toml").expect("written config should exist");
        let roles = role_names(&actual, "observers");
        assert_eq!(roles, vec!["consistency".to_string(), "librarian".to_string()]);
    }

    #[test]
    fn ensure_role_scheduled_adds_missing_innovators_section() {
        let repository = TestStore::new().with_file(
            ".jlo/config.toml",
            r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"
"#,
        );

        let updated = ensure_role_scheduled(
            &repository,
            Layer::Innovators,
            &RoleId::new("librarian").expect("valid role id"),
        )
        .expect("schedule update should succeed");
        assert!(updated);

        let actual = repository.read_file(".jlo/config.toml").expect("written config should exist");
        let roles = role_names(&actual, "innovators");
        assert_eq!(roles, vec!["librarian".to_string()]);
    }
}
