use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::{
    ensure_int, ensure_non_empty_sequence, ensure_non_empty_string, get_sequence, get_string,
    load_yaml_mapping,
};

use crate::app::commands::doctor::schemas::dates::ensure_date;

pub fn validate_observer_perspective(path: &Path, role_name: &str, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_observer_perspective_data(&data, path, role_name, diagnostics);
}

pub fn validate_observer_perspective_data(
    data: &Mapping,
    path: &Path,
    role_name: &str,
    diagnostics: &mut Diagnostics,
) {
    ensure_int(data, path, "schema_version", diagnostics, Some(2));
    ensure_non_empty_string(data, path, "role", diagnostics);
    ensure_date(data, path, "updated_at", diagnostics);
    ensure_non_empty_sequence(data, path, "goals", diagnostics);

    if let Some(focus_paths) = get_sequence(data, "focus_paths") {
        for (idx, item) in focus_paths.iter().enumerate() {
            if !item.is_string() {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("focus_paths[{}] must be a string", idx),
                );
            }
        }
    } else if data.get("focus_paths").is_some() {
        diagnostics.push_error(path.display().to_string(), "focus_paths must be a sequence");
    }

    // rules can be empty initially
    if let Some(rules) = get_sequence(data, "rules") {
        for (idx, item) in rules.iter().enumerate() {
            if !item.is_string() {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("rules[{}] must be a string", idx),
                );
            }
        }
    } else if data.get("rules").is_some() {
        diagnostics.push_error(path.display().to_string(), "rules must be a sequence");
    }

    if let Some(ignore) = get_sequence(data, "ignore") {
        for (idx, item) in ignore.iter().enumerate() {
            if !item.is_string() {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("ignore[{}] must be a string", idx),
                );
            }
        }
    } else if data.get("ignore").is_some() {
        diagnostics.push_error(path.display().to_string(), "ignore must be a sequence");
    }

    let role_value = get_string(data, "role").unwrap_or_default();
    if !role_value.is_empty() && role_value != role_name {
        diagnostics.push_error(
            path.display().to_string(),
            format!("role '{}' does not match directory '{}'", role_value, role_name),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_observer_perspective_valid() {
        let yaml = r#"
schema_version: 2
role: "cli_sentinel"
updated_at: "2023-10-27"
focus_paths: ["src/app", "src/assets/prompt-assemble/observers"]
goals: ["Monitor CLI"]
rules: ["Be nice"]
ignore: ["ignore_me"]
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_observer_perspective_invalid_dates() {
        let yaml = r#"
schema_version: 2
role: "cli_sentinel"
updated_at: "invalid-date"
goals: ["Monitor CLI"]
rules: []
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert!(diagnostics.error_count() >= 1);
        let messages: Vec<_> = diagnostics.errors().iter().map(|e| &e.message).collect();
        assert!(messages.iter().any(|m| m.contains("updated_at must be YYYY-MM-DD")));
    }

    #[test]
    fn test_validate_observer_perspective_missing_fields() {
        let yaml = r#"
schema_version: 2
role: "cli_sentinel"
updated_at: "2023-10-27"
# Missing goals, rules
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
    }

    #[test]
    fn test_validate_observer_perspective_focus_paths_not_sequence() {
        let yaml = r#"
schema_version: 2
role: "cli_sentinel"
updated_at: "2023-10-27"
focus_paths: "not a sequence"
goals: ["Monitor CLI"]
rules: ["Be nice"]
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_observer_perspective_data(&data, &path, "cli_sentinel", &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        assert!(diagnostics.errors()[0].message.contains("focus_paths must be a sequence"));
    }
}
