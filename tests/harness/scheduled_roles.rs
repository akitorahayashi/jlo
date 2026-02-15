use std::fs;
use std::path::Path;
use toml::Value;

pub(crate) fn read_scheduled_role_names(root: &Path, layer: &str) -> Vec<String> {
    let content = fs::read_to_string(root.join(".jlo/config.toml")).expect("read config");
    let value: Value = toml::from_str(&content).expect("parse config");

    let roles = value
        .get(layer)
        .and_then(|layer_value| layer_value.get("roles"))
        .and_then(|roles_value| roles_value.as_array())
        .cloned()
        .unwrap_or_default();

    roles
        .into_iter()
        .filter_map(|role_value| {
            role_value.get("name").and_then(|name| name.as_str()).map(|name| name.to_string())
        })
        .collect()
}
