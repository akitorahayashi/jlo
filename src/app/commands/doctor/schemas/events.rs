use serde_yaml::Mapping;
use std::path::Path;

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::{
    ensure_enum, ensure_id, ensure_int, ensure_non_empty_string, get_sequence, get_string,
    load_yaml_mapping,
};

use super::dates::ensure_date;

pub fn validate_event_file(
    path: &Path,
    state: &str,
    event_confidence: &[String],
    diagnostics: &mut Diagnostics,
) {
    let data = match load_yaml_mapping(path, diagnostics) {
        Some(data) => data,
        None => return,
    };
    validate_event(&data, path, state, event_confidence, diagnostics);
}

pub fn validate_event(
    data: &Mapping,
    path: &Path,
    state: &str,
    event_confidence: &[String],
    diagnostics: &mut Diagnostics,
) {
    ensure_int(data, path, "schema_version", diagnostics, Some(1));
    ensure_id(data, path, "id", diagnostics);

    let requirement_id = match get_string(data, "requirement_id") {
        Some(value) => value,
        None => {
            diagnostics.push_error(path.display().to_string(), "requirement_id is required");
            return;
        }
    };
    if state == "pending" && !requirement_id.is_empty() {
        diagnostics
            .push_error(path.display().to_string(), "requirement_id must be empty in pending");
    }
    if state == "decided" && requirement_id.is_empty() {
        diagnostics.push_error(path.display().to_string(), "requirement_id must be set in decided");
    }

    ensure_date(data, path, "created_at", diagnostics);
    ensure_non_empty_string(data, path, "author_role", diagnostics);
    let allowed: Vec<&str> = event_confidence.iter().map(|value| value.as_str()).collect();
    ensure_enum(data, path, "confidence", &allowed, diagnostics);
    ensure_non_empty_string(data, path, "title", diagnostics);
    ensure_non_empty_string(data, path, "statement", diagnostics);

    if let Some(evidence) = get_sequence(data, "evidence") {
        if evidence.is_empty() {
            diagnostics.push_error(path.display().to_string(), "evidence must have entries");
        } else {
            for (idx, entry) in evidence.iter().enumerate() {
                if let serde_yaml::Value::Mapping(map) = entry {
                    if get_string(map, "path").unwrap_or_default().is_empty() {
                        diagnostics.push_error(
                            path.display().to_string(),
                            format!("evidence[{}].path is required", idx),
                        );
                    }
                    if get_sequence(map, "loc").map(|seq| seq.is_empty()).unwrap_or(true) {
                        diagnostics.push_error(
                            path.display().to_string(),
                            format!("evidence[{}].loc is required", idx),
                        );
                    }
                    if get_string(map, "note").unwrap_or_default().is_empty() {
                        diagnostics.push_error(
                            path.display().to_string(),
                            format!("evidence[{}].note is required", idx),
                        );
                    }
                } else {
                    diagnostics.push_error(
                        path.display().to_string(),
                        format!("evidence[{}] must be a map", idx),
                    );
                }
            }
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "Missing evidence list");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_event_data_valid() {
        let yaml = r#"
schema_version: 1
id: "abc123"
requirement_id: ""
created_at: "2023-10-27"
author_role: "observer"
confidence: "high"
title: "Something happened"
statement: "Evidence suggests..."
evidence:
  - path: "src/main.rs"
    loc: ["10-20"]
    note: "See this"
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let confidence = vec!["high".to_string(), "low".to_string()];

        validate_event(&data, &path, "pending", &confidence, &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_event_data_invalid_state() {
        let yaml = r#"
schema_version: 1
id: "abc123"
requirement_id: "xyz789"  # Should be empty for pending
created_at: "2023-10-27"
author_role: "observer"
confidence: "high"
title: "Something happened"
statement: "Evidence suggests..."
evidence: []
"#;
        let data: Mapping = serde_yaml::from_str(yaml).unwrap();
        let path = PathBuf::from("test.yml");
        let mut diagnostics = Diagnostics::default();
        let confidence = vec!["high".to_string()];

        validate_event(&data, &path, "pending", &confidence, &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        // Should have errors: requirement_id must be empty in pending, evidence must have entries
    }
}
