use std::path::{Path, PathBuf};

use crate::ports::WorkspaceStore;

use super::diagnostics::Diagnostics;
use super::yaml::is_kebab_case;

pub fn naming_checks(
    store: &impl WorkspaceStore,
    jules_path: &Path,
    workstreams: &[String],
    issue_labels: &[String],
    event_states: &[String],
    diagnostics: &mut Diagnostics,
) {
    for workstream in workstreams {
        let ws_dir = jules_path.join("workstreams").join(workstream);
        let exchange_dir = ws_dir.join("exchange");

        let events_dir = exchange_dir.join("events");
        for state in event_states {
            for entry in list_files(store, &events_dir.join(state), diagnostics) {
                validate_filename(&entry, diagnostics, "event");
            }
        }

        let issues_dir = exchange_dir.join("issues");
        for label in issue_labels {
            for entry in list_files(store, &issues_dir.join(label), diagnostics) {
                validate_filename(&entry, diagnostics, "issue");
            }
        }
    }
}

fn validate_filename(path: &Path, diagnostics: &mut Diagnostics, kind: &str) {
    if path.file_name().and_then(|name| name.to_str()) == Some(".gitkeep") {
        return;
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
        diagnostics.push_error(path.display().to_string(), format!("{} file must be .yml", kind));
        return;
    }

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if !is_kebab_case(file_stem) {
        diagnostics.push_error(
            path.display().to_string(),
            format!("{} filename must be kebab-case", kind),
        );
    }
}

fn list_files(
    store: &impl WorkspaceStore,
    dir: &Path,
    diagnostics: &mut Diagnostics,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let dir_str = dir.to_str().unwrap();
    match store.list_dir(dir_str) {
        Ok(entries) => {
            for entry in entries {
                // entry is PathBuf, full path usually
                if !store.is_dir(entry.to_str().unwrap()) {
                    files.push(entry);
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::app::commands::doctor::diagnostics::Diagnostics;

    use super::*;

    #[test]
    fn test_validate_filename_valid_case() {
        let mut diagnostics = Diagnostics::default();
        validate_filename(&PathBuf::from("valid-name.yml"), &mut diagnostics, "test");
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_filename_ignores_gitkeep() {
        let mut diagnostics = Diagnostics::default();
        validate_filename(&PathBuf::from(".gitkeep"), &mut diagnostics, "test");
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_filename_invalid_extension() {
        let mut diagnostics = Diagnostics::default();
        validate_filename(&PathBuf::from("valid-name.txt"), &mut diagnostics, "test");
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("must be .yml"));
    }

    #[test]
    fn test_validate_filename_invalid_camel_case() {
        let mut diagnostics = Diagnostics::default();
        validate_filename(&PathBuf::from("InvalidName.yml"), &mut diagnostics, "test");
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("must be kebab-case"));
    }

    #[test]
    fn test_validate_filename_invalid_snake_case() {
        let mut diagnostics = Diagnostics::default();
        validate_filename(&PathBuf::from("invalid_name.yml"), &mut diagnostics, "test");
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("must be kebab-case"));
    }

    #[test]
    fn test_validate_filename_invalid_characters() {
        let mut diagnostics = Diagnostics::default();
        validate_filename(&PathBuf::from("invalid@name.yml"), &mut diagnostics, "test");
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("must be kebab-case"));
    }
}
