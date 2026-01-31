use include_dir::{Dir, DirEntry, include_dir};

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

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("{}:", key)) {
            let value = trimmed
                .split_once(':')
                .map(|(_, segment)| segment.split('#').next().unwrap_or("").trim())
                .unwrap_or("");
            let cleaned = value.trim_matches('"').trim_matches('\'');
            let mut values: Vec<String> = if cleaned.contains('|') {
                cleaned.split('|').map(|part| part.trim().to_string()).collect()
            } else if cleaned.is_empty() {
                Vec::new()
            } else {
                vec![cleaned.to_string()]
            };
            values.retain(|item| !item.is_empty());
            if values.is_empty() {
                return Err(AppError::config_error(format!(
                    "No enum values found for {} in {}",
                    key, path
                )));
            }
            return Ok(values);
        }
    }

    Err(AppError::config_error(format!("Enum key '{}' not found in scaffold file {}", key, path)))
}
