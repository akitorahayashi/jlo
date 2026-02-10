use include_dir::{Dir, DirEntry, include_dir};
use serde_yaml::Value;

use crate::domain::AppError;

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

pub fn list_issue_labels() -> Result<Vec<String>, AppError> {
    let issues_dir = SCAFFOLD_DIR
        .get_dir(".jules/exchange/issues")
        .ok_or_else(|| AppError::InternalError("Missing scaffold issues directory".into()))?;

    let mut labels = Vec::new();
    for entry in issues_dir.entries() {
        if let DirEntry::Dir(subdir) = entry
            && let Some(name) = subdir.path().file_name()
        {
            labels.push(name.to_string_lossy().to_string());
        }
    }

    labels.sort();
    Ok(labels)
}

pub fn list_event_states() -> Result<Vec<String>, AppError> {
    let events_dir = SCAFFOLD_DIR
        .get_dir(".jules/exchange/events")
        .ok_or_else(|| AppError::InternalError("Missing scaffold events directory".into()))?;

    let mut states = Vec::new();
    for entry in events_dir.entries() {
        if let DirEntry::Dir(subdir) = entry
            && let Some(name) = subdir.path().file_name()
        {
            states.push(name.to_string_lossy().to_string());
        }
    }

    states.sort();
    Ok(states)
}

pub fn scaffold_file_content(path: &str) -> Option<String> {
    SCAFFOLD_DIR.get_file(path).and_then(|file| file.contents_utf8()).map(|s| s.to_string())
}

pub fn read_enum_values(path: &str, key: &str) -> Result<Vec<String>, AppError> {
    let content = scaffold_file_content(path)
        .ok_or_else(|| AppError::InternalError(format!("Missing scaffold file: {}", path)))?;

    parse_enum_values_from_content(&content, key, path)
}

pub fn parse_enum_values_from_content(
    content: &str,
    key: &str,
    path_context: &str,
) -> Result<Vec<String>, AppError> {
    let value: Value = serde_yaml::from_str(content).map_err(|err| {
        AppError::InternalError(format!("Failed to parse {}: {}", path_context, err))
    })?;

    let map = match value {
        Value::Mapping(map) => map,
        _ => {
            return Err(AppError::InternalError(format!(
                "Expected root mapping in {}",
                path_context
            )));
        }
    };

    let value_str =
        map.get(Value::String(key.to_string())).and_then(|value| value.as_str()).unwrap_or("");

    let values: Vec<String> = value_str
        .split('|')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect();

    if values.is_empty() {
        return Err(AppError::InternalError(format!(
            "No enum values found for {} in {}",
            key, path_context
        )));
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enum_values_valid() {
        let content = "my_key: val1 | val2 | val3";
        let values = parse_enum_values_from_content(content, "my_key", "test.yml").unwrap();
        assert_eq!(values, vec!["val1", "val2", "val3"]);
    }

    #[test]
    fn test_parse_enum_values_whitespace() {
        let content = "my_key:  val1  |  val2  ";
        let values = parse_enum_values_from_content(content, "my_key", "test.yml").unwrap();
        assert_eq!(values, vec!["val1", "val2"]);
    }

    #[test]
    fn test_parse_enum_values_missing_key() {
        let content = "other_key: val1";
        let err = parse_enum_values_from_content(content, "my_key", "test.yml").unwrap_err();
        match err {
            AppError::InternalError(msg) => {
                assert!(msg.contains("No enum values found for my_key in test.yml"));
            }
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_parse_enum_values_empty_value() {
        let content = "my_key: ";
        let err = parse_enum_values_from_content(content, "my_key", "test.yml").unwrap_err();
        match err {
            AppError::InternalError(msg) => {
                assert!(msg.contains("No enum values found for my_key in test.yml"));
            }
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_parse_enum_values_invalid_yaml() {
        let content = ": invalid yaml";
        let err = parse_enum_values_from_content(content, "my_key", "test.yml").unwrap_err();
        match err {
            AppError::InternalError(msg) => {
                assert!(msg.contains("Failed to parse test.yml"));
            }
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_parse_enum_values_not_a_mapping() {
        let content = "- list item";
        let err = parse_enum_values_from_content(content, "my_key", "test.yml").unwrap_err();
        match err {
            AppError::InternalError(msg) => {
                assert!(msg.contains("Expected root mapping in test.yml"));
            }
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_scaffold_assets_integrity() {
        // Ensure the directory is not empty
        assert!(!SCAFFOLD_DIR.entries().is_empty(), "Scaffold directory should not be empty");

        for entry in SCAFFOLD_DIR.entries() {
            check_entry(entry);
        }
    }

    fn check_entry(entry: &DirEntry) {
        match entry {
            DirEntry::File(file) => {
                // Ensure we can read the content
                // We allow empty files if they are explicitly placeholders (like .gitkeep), but generally assets should be non-empty.
                // For now, just checking we can access it is enough to verify inclusion.
                let path = file.path().to_string_lossy();
                if !path.ends_with(".gitkeep") {
                    assert!(!file.contents().is_empty(), "File {} is empty", path);
                }
            }
            DirEntry::Dir(dir) => {
                for entry in dir.entries() {
                    check_entry(entry);
                }
            }
        }
    }
}
