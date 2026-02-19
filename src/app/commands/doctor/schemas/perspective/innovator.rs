use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::{
    ensure_int, ensure_non_empty_string, get_string, load_yaml_mapping,
};

pub fn validate_innovator_perspective(path: &Path, role_name: &str, diagnostics: &mut Diagnostics) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_innovator_perspective_data(&data, path, role_name, diagnostics);
}

pub fn validate_innovator_perspective_data(
    data: &Mapping,
    path: &Path,
    role_name: &str,
    diagnostics: &mut Diagnostics,
) {
    ensure_int(data, path, "schema_version", diagnostics, Some(1));
    ensure_non_empty_string(data, path, "role", diagnostics);
    ensure_non_empty_string(data, path, "focus", diagnostics);

    if data.get("feedback_assimilation").is_some() {
        diagnostics.push_error(
            path.display().to_string(),
            "feedback_assimilation is deprecated for innovator perspective and must be removed",
        );
    }

    if data.get("recent_proposals").is_some() {
        diagnostics.push_error(
            path.display().to_string(),
            "recent_proposals is deprecated for innovator perspective and must be removed",
        );
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
    fn test_validate_innovator_perspective_valid() {
        let yaml = r#"
schema_version: 1
role: "architect"
focus: "High-leverage intervention design"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_innovator_perspective_data(&data, &path, "architect", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_innovator_perspective_deprecated_fields() {
        let yaml = r#"
schema_version: 1
role: "architect"
focus: "High-leverage intervention design"
feedback_assimilation:
  observer_inputs: []
recent_proposals:
  - "Proposal A"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("perspective.yml");
        let mut diagnostics = Diagnostics::default();

        validate_innovator_perspective_data(&data, &path, "architect", &mut diagnostics);
        assert!(diagnostics.error_count() >= 2);
        let messages: Vec<_> = diagnostics.errors().iter().map(|e| &e.message).collect();
        assert!(
            messages.iter().any(
                |m| m.contains("feedback_assimilation is deprecated for innovator perspective")
            )
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("recent_proposals is deprecated for innovator perspective"))
        );
    }
}
