use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::config::parse::parse_config_content;
use crate::domain::{AppError, ControlPlaneConfig, Layer, Version};

use super::diagnostics::Diagnostics;

pub fn read_control_plane_config(
    root: &Path,
    diagnostics: &mut Diagnostics,
) -> Result<ControlPlaneConfig, AppError> {
    let config_path = crate::domain::config::paths::config(root);
    if !config_path.exists() {
        diagnostics.push_error(config_path.display().to_string(), "Missing .jlo/config.toml");
        return Ok(ControlPlaneConfig::default());
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics.push_error(config_path.display().to_string(), err.to_string());
            return Ok(ControlPlaneConfig::default());
        }
    };

    match parse_config_content(&content) {
        Ok(config) => Ok(config),
        Err(err) => {
            diagnostics.push_error(config_path.display().to_string(), err.to_string());
            Ok(ControlPlaneConfig::default())
        }
    }
}

pub struct StructuralInputs<'a> {
    pub jules_path: &'a Path,
    pub root: &'a Path,
    pub event_states: &'a [String],
}

pub fn structural_checks(inputs: StructuralInputs<'_>, diagnostics: &mut Diagnostics) {
    ensure_file_exists(&crate::domain::jules_paths::jules_readme(inputs.root), diagnostics);
    ensure_file_exists(&crate::domain::jules_paths::project_readme(inputs.root), diagnostics);
    ensure_file_exists(&crate::domain::config::paths::config(inputs.root), diagnostics);
    ensure_file_exists(&crate::domain::jules_paths::version_file(inputs.root), diagnostics);

    check_version_file(inputs.jules_path, env!("CARGO_PKG_VERSION"), diagnostics);

    for layer in Layer::ALL {
        // Check schemas/ directory (layers with output schemas: .jules/schemas/<layer>/)
        if layer.has_schemas() {
            let schemas_dir = crate::domain::layers::paths::schemas_dir(inputs.jules_path, layer);
            if !schemas_dir.exists() {
                diagnostics
                    .push_error(schemas_dir.display().to_string(), "Missing schemas directory");
            }
        }

        if layer.is_single_role() {
            // Narrator requires changes.yml schema template
            if layer == Layer::Narrator {
                let change_template =
                    crate::domain::layers::paths::narrator_change_schema(inputs.jules_path);
                if !change_template.exists() {
                    diagnostics
                        .push_error(change_template.display().to_string(), "Missing changes.yml");
                }
            }
        } else {
            // Check roles/ container directory for multi-role layers in .jlo/
            let jlo_layer_dir = crate::domain::roles::paths::layer_dir(inputs.root, layer);

            if jlo_layer_dir.exists() {
                for entry in list_subdirs(&jlo_layer_dir, diagnostics) {
                    let role_file = entry.join("role.yml");
                    if !role_file.exists() {
                        diagnostics.push_error(role_file.display().to_string(), "Missing role.yml");
                    }
                }
            }
        }
    }

    // Flat exchange directory structure
    {
        ensure_directory_exists(
            crate::domain::exchange::paths::exchange_dir(inputs.jules_path),
            diagnostics,
        );

        ensure_directory_exists(
            crate::domain::exchange::events::paths::events_dir(inputs.jules_path),
            diagnostics,
        );
        for state in inputs.event_states {
            ensure_directory_exists(
                crate::domain::exchange::events::paths::events_state_dir(inputs.jules_path, state),
                diagnostics,
            );
        }

        ensure_directory_exists(
            crate::domain::exchange::requirements::paths::requirements_dir(inputs.jules_path),
            diagnostics,
        );

        ensure_directory_exists(
            crate::domain::exchange::proposals::paths::proposals_dir(inputs.jules_path),
            diagnostics,
        );
    }
}

fn check_version_file(jules_path: &Path, current_version: &str, diagnostics: &mut Diagnostics) {
    let version_path = jules_path.join(crate::domain::VERSION_FILE);
    if !version_path.exists() {
        return;
    }

    let content = match fs::read_to_string(&version_path) {
        Ok(content) => content.trim().to_string(),
        Err(err) => {
            diagnostics.push_error(version_path.display().to_string(), err.to_string());
            return;
        }
    };

    let Some(runtime_version) = Version::parse(&content) else {
        diagnostics.push_error(version_path.display().to_string(), "Invalid version format");
        return;
    };

    let Some(current_version_obj) = Version::parse(current_version) else {
        diagnostics
            .push_error(version_path.display().to_string(), "Current binary version is invalid");
        return;
    };

    if runtime_version > current_version_obj {
        diagnostics.push_error(
            version_path.display().to_string(),
            "Repository version is newer than the binary",
        );
    }
}

fn ensure_directory_exists(path: PathBuf, diagnostics: &mut Diagnostics) {
    if !path.exists() {
        diagnostics.push_error(path.display().to_string(), "Missing directory");
    }
}

fn ensure_file_exists(path: &Path, diagnostics: &mut Diagnostics) {
    if !path.exists() {
        diagnostics.push_error(path.display().to_string(), "Missing required file");
    }
}

pub fn list_subdirs(path: &Path, diagnostics: &mut Diagnostics) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            dirs.push(entry_path);
                        }
                    }
                    Err(err) => {
                        diagnostics.push_error(
                            path.display().to_string(),
                            format!("Failed to read directory entry: {}", err),
                        );
                    }
                }
            }
        }
        Err(err) => {
            diagnostics.push_error(
                path.display().to_string(),
                format!("Failed to read directory: {}", err),
            );
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;

    use crate::app::commands::doctor::diagnostics::Diagnostics;

    use super::*;

    #[test]
    fn test_check_version_file_missing_is_ok() {
        let temp = assert_fs::TempDir::new().unwrap();
        let mut diagnostics = Diagnostics::default();
        check_version_file(temp.path(), "1.0.0", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_check_version_file_match_is_ok() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child(".jlo-version").write_str("1.0.0").unwrap();
        let mut diagnostics = Diagnostics::default();
        check_version_file(temp.path(), "1.0.0", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_check_version_file_repository_older_is_ok() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child(".jlo-version").write_str("0.9.0").unwrap();
        let mut diagnostics = Diagnostics::default();
        check_version_file(temp.path(), "1.0.0", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_check_version_file_repository_newer_is_error() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child(".jlo-version").write_str("2.0.0").unwrap();
        let mut diagnostics = Diagnostics::default();
        check_version_file(temp.path(), "1.0.0", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
        assert!(
            diagnostics.errors()[0].message.contains("Repository version is newer than the binary")
        );
    }

    #[test]
    fn test_check_version_file_invalid_format_is_error() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child(".jlo-version").write_str("invalid").unwrap();
        let mut diagnostics = Diagnostics::default();
        check_version_file(temp.path(), "1.0.0", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
        assert!(diagnostics.errors()[0].message.contains("Invalid version format"));
    }

    fn create_valid_repository(temp: &assert_fs::TempDir) {
        temp.child(".jules/JULES.md").touch().unwrap();
        temp.child(".jules/README.md").touch().unwrap();
        temp.child(".jlo/config.toml").touch().unwrap();
        temp.child(".jules/.jlo-version").write_str(env!("CARGO_PKG_VERSION")).unwrap();

        // Schemas for layers that have them (.jules/schemas/<layer>/)
        for layer in Layer::ALL {
            if layer.has_schemas() {
                let schemas_dir = temp.child(format!(".jules/schemas/{}", layer.dir_name()));
                schemas_dir.create_dir_all().unwrap();
            }

            if layer == Layer::Narrator {
                temp.child(".jules/schemas/narrator/changes.yml").touch().unwrap();
            }

            if !layer.is_single_role() {
                // Multi-role layers have role definitions in .jlo/roles
                let jlo_role_dir = temp.child(format!(".jlo/roles/{}/my-role", layer.dir_name()));
                jlo_role_dir.create_dir_all().unwrap();
                jlo_role_dir.child("role.yml").touch().unwrap();
            }
        }

        // Flat exchange structure in .jules/
        let exchange = temp.child(".jules/exchange");
        exchange.child("events").create_dir_all().unwrap();
        exchange.child("requirements").create_dir_all().unwrap();

        // We need to match inputs for event states
        exchange.child("events/pending").create_dir_all().unwrap();

        // Proposal exchange directory
        exchange.child("proposals").create_dir_all().unwrap();
    }

    #[test]
    fn test_structural_checks_success() {
        let temp = assert_fs::TempDir::new().unwrap();
        create_valid_repository(&temp);

        let mut diagnostics = Diagnostics::default();
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            event_states: &event_states,
        };

        structural_checks(inputs, &mut diagnostics);
        assert_eq!(
            diagnostics.error_count(),
            0,
            "Expected 0 errors, got: {:?}",
            diagnostics.errors()
        );
    }

    #[test]
    fn test_structural_checks_missing_critical_files() {
        let temp = assert_fs::TempDir::new().unwrap();
        create_valid_repository(&temp);

        // Remove config.toml
        std::fs::remove_file(temp.path().join(".jlo/config.toml")).unwrap();
        // Remove JULES.md
        std::fs::remove_file(temp.path().join(".jules/JULES.md")).unwrap();

        let mut diagnostics = Diagnostics::default();
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            event_states: &event_states,
        };

        structural_checks(inputs, &mut diagnostics);

        // Expect at least 2 errors
        assert!(diagnostics.error_count() >= 2);
        let errors: Vec<String> = diagnostics.errors().iter().map(|e| e.message.clone()).collect();
        assert!(errors.contains(&"Missing required file".to_string()));
    }

    #[test]
    fn test_structural_checks_missing_schemas_dir() {
        let temp = assert_fs::TempDir::new().unwrap();
        create_valid_repository(&temp);

        // Remove narrator schemas directory
        std::fs::remove_dir_all(temp.path().join(".jules/schemas/narrator")).unwrap();

        let mut diagnostics = Diagnostics::default();
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            event_states: &event_states,
        };

        structural_checks(inputs, &mut diagnostics);

        assert!(diagnostics.error_count() >= 1);
        let errors: Vec<String> = diagnostics.errors().iter().map(|e| e.message.clone()).collect();
        assert!(errors.iter().any(|msg| msg.contains("Missing")));
    }
}
