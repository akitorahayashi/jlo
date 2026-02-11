use std::path::Path;

use crate::domain::workspace::paths::jules;

use super::diagnostics::Diagnostics;
use super::structure::list_subdirs;
use super::yaml::is_kebab_case;

pub fn naming_checks(jules_path: &Path, event_states: &[String], diagnostics: &mut Diagnostics) {
    for state in event_states {
        for entry in list_files(&jules::events_state_dir(jules_path, state), diagnostics) {
            validate_filename(&entry, diagnostics, "event");
        }
    }

    for entry in list_files(&jules::requirements_dir(jules_path), diagnostics) {
        validate_filename(&entry, diagnostics, "requirement");
    }

    let innovators_dir = jules::innovators_dir(jules_path);
    if innovators_dir.exists() {
        for persona_dir in list_subdirs(&innovators_dir, diagnostics) {
            let comments_dir = persona_dir.join("comments");
            for entry in list_files(&comments_dir, diagnostics) {
                validate_filename(&entry, diagnostics, "innovator comment");
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

fn list_files(dir: &Path, diagnostics: &mut Diagnostics) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
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
