use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use super::diagnostics::Diagnostics;

pub fn load_yaml_mapping(path: &Path, diagnostics: &mut Diagnostics) -> Option<Mapping> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics.push_error(path.display().to_string(), err.to_string());
            return None;
        }
    };

    match serde_yaml::from_str::<serde_yaml::Value>(&content) {
        Ok(serde_yaml::Value::Mapping(map)) => Some(map),
        Ok(_) => {
            diagnostics.push_error(path.display().to_string(), "YAML root is not a mapping");
            None
        }
        Err(err) => {
            diagnostics.push_error(path.display().to_string(), err.to_string());
            None
        }
    }
}

pub fn get_string(map: &Mapping, key: &str) -> Option<String> {
    map.get(serde_yaml::Value::String(key.to_string())).and_then(|value| match value {
        serde_yaml::Value::String(value) => Some(value.clone()),
        _ => None,
    })
}

pub fn get_bool(map: &Mapping, key: &str) -> Option<bool> {
    map.get(serde_yaml::Value::String(key.to_string())).and_then(|value| match value {
        serde_yaml::Value::Bool(value) => Some(*value),
        _ => None,
    })
}

pub fn get_sequence(map: &Mapping, key: &str) -> Option<Vec<serde_yaml::Value>> {
    map.get(serde_yaml::Value::String(key.to_string())).and_then(|value| match value {
        serde_yaml::Value::Sequence(values) => Some(values.clone()),
        _ => None,
    })
}

pub fn get_sequence_strings(map: &Mapping, key: &str) -> Vec<String> {
    get_sequence(map, key)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| match value {
            serde_yaml::Value::String(text) => Some(text),
            _ => None,
        })
        .collect()
}

pub fn ensure_non_empty_sequence(
    map: &Mapping,
    path: &Path,
    key: &str,
    diagnostics: &mut Diagnostics,
) {
    if get_sequence(map, key).map(|seq| seq.is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), format!("{} must have entries", key));
    }
}

pub fn ensure_non_empty_string(
    map: &Mapping,
    path: &Path,
    key: &str,
    diagnostics: &mut Diagnostics,
) {
    if get_string(map, key).map(|value| value.trim().is_empty()).unwrap_or(true) {
        diagnostics.push_error(path.display().to_string(), format!("{} is required", key));
    }
}

pub fn ensure_int(
    map: &Mapping,
    path: &Path,
    key: &str,
    diagnostics: &mut Diagnostics,
    expected: Option<i64>,
) {
    let value = map.get(serde_yaml::Value::String(key.to_string()));
    let number = match value {
        Some(serde_yaml::Value::Number(number)) => number.as_i64(),
        _ => None,
    };

    match number {
        Some(actual) => {
            if let Some(expected) = expected
                && actual != expected
            {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("{} must be {}", key, expected),
                );
            }
        }
        None => {
            diagnostics.push_error(path.display().to_string(), format!("{} is required", key));
        }
    }
}

pub fn ensure_enum(
    map: &Mapping,
    path: &Path,
    key: &str,
    allowed: &[&str],
    diagnostics: &mut Diagnostics,
) {
    let value = get_string(map, key).unwrap_or_default();
    if value.trim().is_empty() {
        diagnostics.push_error(path.display().to_string(), format!("{} is required", key));
        return;
    }

    if !allowed.is_empty() && !allowed.contains(&value.as_str()) {
        diagnostics.push_error(path.display().to_string(), format!("{} is invalid", key));
    }
}

pub fn ensure_id(map: &Mapping, path: &Path, key: &str, diagnostics: &mut Diagnostics) {
    let value = get_string(map, key).unwrap_or_default();
    if !is_valid_id(&value) {
        diagnostics.push_error(
            path.display().to_string(),
            format!("{} must be 6 lowercase alphanumeric chars", key),
        );
    }
}

pub fn read_yaml_files(dir: &Path, diagnostics: &mut Diagnostics) -> Vec<PathBuf> {
    let mut files = Vec::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file()
                            && path.extension().and_then(|ext| ext.to_str()) == Some("yml")
                        {
                            files.push(path);
                        }
                    }
                    Err(err) => {
                        diagnostics.push_error(
                            dir.display().to_string(),
                            format!("Failed to read directory entry: {}", err),
                        );
                    }
                }
            }
        }
        Err(err) => {
            diagnostics.push_error(
                dir.display().to_string(),
                format!("Failed to read directory: {}", err),
            );
        }
    }
    files
}

pub fn read_yaml_string(path: &Path, key: &str, diagnostics: &mut Diagnostics) -> Option<String> {
    let map = load_yaml_mapping(path, diagnostics)?;
    get_string(&map, key)
}

pub fn read_yaml_strings(
    path: &Path,
    key: &str,
    diagnostics: &mut Diagnostics,
) -> Option<Vec<String>> {
    let map = load_yaml_mapping(path, diagnostics)?;
    Some(get_sequence_strings(&map, key))
}

pub fn read_yaml_bool(path: &Path, key: &str, diagnostics: &mut Diagnostics) -> Option<bool> {
    let map = load_yaml_mapping(path, diagnostics)?;
    get_bool(&map, key)
}

pub fn is_valid_id(value: &str) -> bool {
    value.len() == 6 && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

pub fn is_kebab_case(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.starts_with('-') || value.ends_with('-') {
        return false;
    }
    let mut prev_dash = false;
    for ch in value.chars() {
        let is_valid = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-';
        if !is_valid {
            return false;
        }
        if ch == '-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::app::commands::doctor::diagnostics::Diagnostics;

    use super::*;

    #[test]
    fn test_is_valid_id() {
        assert!(is_valid_id("abc123"));
        assert!(!is_valid_id("abc")); // Too short
        assert!(!is_valid_id("abc1234")); // Too long
        assert!(!is_valid_id("ABC123")); // Uppercase
        assert!(!is_valid_id("abc-12")); // Special char
    }

    #[test]
    fn test_is_kebab_case() {
        assert!(is_kebab_case("valid-name"));
        assert!(is_kebab_case("valid"));
        assert!(!is_kebab_case("Invalid")); // Uppercase
        assert!(!is_kebab_case("invalid_name")); // Underscore
        assert!(!is_kebab_case("-invalid")); // Starts with dash
        assert!(!is_kebab_case("invalid-")); // Ends with dash
        assert!(!is_kebab_case("invalid--name")); // Double dash
        assert!(!is_kebab_case("")); // Empty
    }

    #[test]
    fn test_ensure_non_empty_string() {
        let path = PathBuf::from("test.yml");
        let yaml_str = r#"
            str_key: "value"
            empty_str: ""
        "#;
        let map: Mapping = serde_yaml::from_str::<serde_yaml::Value>(yaml_str)
            .unwrap()
            .as_mapping()
            .unwrap()
            .clone();

        // Valid case
        let mut diagnostics = Diagnostics::default();
        ensure_non_empty_string(&map, &path, "str_key", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);

        // Empty string
        let mut diagnostics = Diagnostics::default();
        ensure_non_empty_string(&map, &path, "empty_str", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);

        // Missing key
        let mut diagnostics = Diagnostics::default();
        ensure_non_empty_string(&map, &path, "missing", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
    }

    #[test]
    fn test_ensure_non_empty_sequence() {
        let path = PathBuf::from("test.yml");
        let yaml_str = r#"
            seq_key: ["a", "b"]
            empty_seq: []
        "#;
        let map: Mapping = serde_yaml::from_str::<serde_yaml::Value>(yaml_str)
            .unwrap()
            .as_mapping()
            .unwrap()
            .clone();

        // Valid case
        let mut diagnostics = Diagnostics::default();
        ensure_non_empty_sequence(&map, &path, "seq_key", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);

        // Empty sequence
        let mut diagnostics = Diagnostics::default();
        ensure_non_empty_sequence(&map, &path, "empty_seq", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
    }

    #[test]
    fn test_ensure_int() {
        let path = PathBuf::from("test.yml");
        let yaml_str = r#"
            int_key: 42
        "#;
        let map: Mapping = serde_yaml::from_str::<serde_yaml::Value>(yaml_str)
            .unwrap()
            .as_mapping()
            .unwrap()
            .clone();

        // Valid case, expected value matches
        let mut diagnostics = Diagnostics::default();
        ensure_int(&map, &path, "int_key", &mut diagnostics, Some(42));
        assert_eq!(diagnostics.error_count(), 0);

        // Expected value mismatch
        let mut diagnostics = Diagnostics::default();
        ensure_int(&map, &path, "int_key", &mut diagnostics, Some(10));
        assert_eq!(diagnostics.error_count(), 1);

        // Missing key
        let mut diagnostics = Diagnostics::default();
        ensure_int(&map, &path, "missing_int", &mut diagnostics, None);
        assert_eq!(diagnostics.error_count(), 1);
    }

    #[test]
    fn test_ensure_enum() {
        let path = PathBuf::from("test.yml");
        let yaml_str = r#"
            str_key: "value"
        "#;
        let map: Mapping = serde_yaml::from_str::<serde_yaml::Value>(yaml_str)
            .unwrap()
            .as_mapping()
            .unwrap()
            .clone();

        // Valid case
        let mut diagnostics = Diagnostics::default();
        ensure_enum(&map, &path, "str_key", &["value", "other"], &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);

        // Invalid value
        let mut diagnostics = Diagnostics::default();
        ensure_enum(&map, &path, "str_key", &["other"], &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
    }

    #[test]
    fn test_ensure_id() {
        let path = PathBuf::from("test.yml");
        let yaml_str = r#"
            valid_id: "abc123"
            invalid_id: "too_short"
        "#;
        let map: Mapping = serde_yaml::from_str::<serde_yaml::Value>(yaml_str)
            .unwrap()
            .as_mapping()
            .unwrap()
            .clone();

        // Valid case
        let mut diagnostics = Diagnostics::default();
        ensure_id(&map, &path, "valid_id", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);

        // Invalid id
        let mut diagnostics = Diagnostics::default();
        ensure_id(&map, &path, "invalid_id", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
    }
}
