use include_dir::{Dir, DirEntry, include_dir};
use serde_yaml::Value;

use crate::domain::AppError;

static SCAFFOLD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/scaffold");

pub fn list_issue_labels() -> Result<Vec<String>, AppError> {
    let issues_dir = SCAFFOLD_DIR
        .get_dir(".jules/workstreams/generic/issues")
        .ok_or_else(|| AppError::config_error("Missing scaffold issues directory"))?;

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
        .get_dir(".jules/workstreams/generic/events")
        .ok_or_else(|| AppError::config_error("Missing scaffold events directory"))?;

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
        .ok_or_else(|| AppError::config_error(format!("Missing scaffold file: {}", path)))?;

    let value: Value = serde_yaml::from_str(&content)
        .map_err(|err| AppError::config_error(format!("Failed to parse {}: {}", path, err)))?;

    let map = match value {
        Value::Mapping(map) => map,
        _ => return Err(AppError::config_error(format!("Expected root mapping in {}", path))),
    };

    let value_str =
        map.get(Value::String(key.to_string())).and_then(|value| value.as_str()).unwrap_or("");

    let values: Vec<String> = value_str
        .split('|')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect();

    if values.is_empty() {
        return Err(AppError::config_error(format!(
            "No enum values found for {} in {}",
            key, path
        )));
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

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
                     assert!(file.contents().len() > 0, "File {} is empty", path);
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
