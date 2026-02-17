use std::path::Path;

use super::diagnostics::Diagnostics;
use super::yaml::is_kebab_case;

pub fn naming_checks(jules_path: &Path, event_states: &[String], diagnostics: &mut Diagnostics) {
    for state in event_states {
        for entry in list_files(
            &crate::domain::exchange::events::paths::events_state_dir(jules_path, state),
            diagnostics,
        ) {
            validate_filename(&entry, diagnostics, "event");
        }
    }

    for entry in list_files(
        &crate::domain::exchange::requirements::paths::requirements_dir(jules_path),
        diagnostics,
    ) {
        validate_filename(&entry, diagnostics, "requirement");
    }

    for entry in list_files(
        &crate::domain::exchange::proposals::paths::proposals_dir(jules_path),
        diagnostics,
    ) {
        validate_proposal_filename(&entry, diagnostics);
    }
}

fn validate_proposal_filename(path: &Path, diagnostics: &mut Diagnostics) {
    if path.file_name().and_then(|name| name.to_str()) == Some(".gitkeep") {
        return;
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
        diagnostics.push_error(path.display().to_string(), "proposal file must be .yml");
        return;
    }

    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        diagnostics.push_error(path.display().to_string(), "proposal filename is invalid");
        return;
    };
    if stem.is_empty() {
        diagnostics.push_error(path.display().to_string(), "proposal filename must not be empty");
        return;
    }

    if !stem.contains('-') {
        diagnostics.push_error(
            path.display().to_string(),
            "proposal filename must include '<role>-<slug>'",
        );
        return;
    } else {
        let parts: Vec<&str> = stem.splitn(2, '-').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            diagnostics.push_error(
                path.display().to_string(),
                "proposal filename must be in the format '<role>-<slug>'",
            );
        }
    }

    if !stem.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
        diagnostics.push_error(
            path.display().to_string(),
            "proposal filename must use kebab-case (lowercase ASCII, digits, or '-')",
        );
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

    #[test]
    fn test_validate_proposal_filename_requires_role_and_slug() {
        let mut diagnostics = Diagnostics::default();
        validate_proposal_filename(&PathBuf::from("invalid-.yml"), &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("<role>-<slug>"));
    }

    #[test]
    fn test_validate_proposal_filename_accepts_valid_pattern() {
        let mut diagnostics = Diagnostics::default();
        validate_proposal_filename(&PathBuf::from("alice-proposal-one.yml"), &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_validate_proposal_filename_rejects_underscores() {
        let mut diagnostics = Diagnostics::default();
        validate_proposal_filename(&PathBuf::from("alice-proposal_one.yml"), &mut diagnostics);
        assert!(diagnostics.error_count() > 0);
        assert!(diagnostics.errors()[0].message.contains("kebab-case"));
    }
}
