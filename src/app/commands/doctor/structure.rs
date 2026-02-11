use std::fs;
use std::path::{Path, PathBuf};

use crate::app::commands::run::parse_config_content;
use crate::domain::workspace::paths::{self, jlo, jules};
use crate::domain::{AppError, Layer, RunConfig};

use super::diagnostics::Diagnostics;

pub fn read_run_config(root: &Path, diagnostics: &mut Diagnostics) -> Result<RunConfig, AppError> {
    let config_path = jlo::config(root);
    if !config_path.exists() {
        diagnostics.push_error(config_path.display().to_string(), "Missing .jlo/config.toml");
        return Ok(RunConfig::default());
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics.push_error(config_path.display().to_string(), err.to_string());
            return Ok(RunConfig::default());
        }
    };

    match parse_config_content(&content) {
        Ok(config) => Ok(config),
        Err(err) => {
            diagnostics.push_error(config_path.display().to_string(), err.to_string());
            Ok(RunConfig::default())
        }
    }
}

pub struct StructuralInputs<'a> {
    pub jules_path: &'a Path,
    pub root: &'a Path,
    pub issue_labels: &'a [String],
    pub event_states: &'a [String],
}

pub fn structural_checks(inputs: StructuralInputs<'_>, diagnostics: &mut Diagnostics) {
    ensure_file_exists(&jules::readme(inputs.root), diagnostics);
    ensure_file_exists(&jules::project_readme(inputs.root), diagnostics);
    ensure_file_exists(&jlo::config(inputs.root), diagnostics);
    ensure_file_exists(&jules::version_file(inputs.root), diagnostics);
    ensure_directory_exists(jules::roles_dir(inputs.jules_path), diagnostics);
    ensure_directory_exists(jlo::roles_dir(inputs.root), diagnostics);
    ensure_file_exists(&jlo::schedule(inputs.root), diagnostics);

    check_version_file(inputs.jules_path, env!("CARGO_PKG_VERSION"), diagnostics);

    for layer in Layer::ALL {
        let layer_dir = jules::layer_dir(inputs.jules_path, layer);
        if !layer_dir.exists() {
            diagnostics.push_error(layer_dir.display().to_string(), "Missing layer directory");
            continue;
        }

        // Phase-specific contracts for layers that use them; single contracts.yml for others
        let contracts = jules::contracts(inputs.jules_path, layer);
        if !contracts.exists() {
            diagnostics.push_error(contracts.display().to_string(), "Missing contracts.yml");
        }

        // Check schemas/ directory (all layers have this except implementers)
        let schemas_dir = jules::schemas_dir(inputs.jules_path, layer);
        if layer != Layer::Implementers && !schemas_dir.exists() {
            diagnostics.push_error(schemas_dir.display().to_string(), "Missing schemas/");
        }

        // Check tasks/ directory (all layers have this)
        let tasks_dir = jules::tasks_dir(inputs.jules_path, layer);
        if !tasks_dir.exists() {
            diagnostics.push_error(tasks_dir.display().to_string(), "Missing tasks/");
        }

        // Check prompt template (all layers have one)
        let prompt_template = jules::prompt_template(inputs.jules_path, layer);
        if !prompt_template.exists() {
            diagnostics.push_error(
                prompt_template.display().to_string(),
                format!("Missing {}", layer.prompt_template_name()),
            );
        }

        if layer.is_single_role() {
            // Narrator requires changes.yml schema template
            if layer == Layer::Narrators {
                let change_template = jules::narrator_change_schema(inputs.jules_path);
                if !change_template.exists() {
                    diagnostics
                        .push_error(change_template.display().to_string(), "Missing changes.yml");
                }
            }
        } else {
            // Check roles/ container directory for multi-role layers in .jlo/
            // The .jules/ structure for multi-role layers (roles/ container) might technically exist from scaffold
            // but the actual role definitions are in .jlo/.
            // structure_checks needs to verify that for every role in .jlo/, it exists.
            let jlo_layer_dir = jlo::layer_dir(inputs.root, layer);

            if !jlo_layer_dir.exists() {
                diagnostics.push_error(
                    jlo_layer_dir.display().to_string(),
                    "Missing .jlo roles directory",
                );
            } else {
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
        ensure_directory_exists(jules::exchange_dir(inputs.jules_path), diagnostics);

        ensure_directory_exists(jules::events_dir(inputs.jules_path), diagnostics);
        for state in inputs.event_states {
            ensure_directory_exists(jules::events_state_dir(inputs.jules_path, state), diagnostics);
        }

        ensure_directory_exists(jules::issues_dir(inputs.jules_path), diagnostics);
        for label in inputs.issue_labels {
            ensure_directory_exists(jules::issues_label_dir(inputs.jules_path, label), diagnostics);
        }

        ensure_directory_exists(jules::workstations_dir(inputs.jules_path), diagnostics);

        let innovators_dir = jules::innovators_dir(inputs.jules_path);
        ensure_directory_exists(innovators_dir.clone(), diagnostics);

        if innovators_dir.exists() {
            for persona_dir in list_subdirs(&innovators_dir, diagnostics) {
                let persona = persona_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let comments_dir = jules::innovator_comments_dir(inputs.jules_path, persona);
                if !comments_dir.exists() {
                    diagnostics.push_error(
                        comments_dir.display().to_string(),
                        "Missing comments/ directory in innovator room",
                    );
                }
            }
        }
    }
}

fn check_version_file(jules_path: &Path, current_version: &str, diagnostics: &mut Diagnostics) {
    let version_path = jules_path.join(paths::VERSION_FILE);
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

    let workspace_parts = parse_version_parts(&content);
    let current_parts = parse_version_parts(current_version);

    if workspace_parts.is_none() {
        diagnostics.push_error(version_path.display().to_string(), "Invalid version format");
        return;
    }

    if current_parts.is_none() {
        diagnostics
            .push_error(version_path.display().to_string(), "Current binary version is invalid");
        return;
    }

    if compare_versions(&workspace_parts.unwrap(), &current_parts.unwrap()) > 0 {
        diagnostics.push_error(
            version_path.display().to_string(),
            "Workspace version is newer than the binary",
        );
    }
}

fn parse_version_parts(version: &str) -> Option<Vec<u32>> {
    let parts: Vec<_> = version.split('.').map(|segment| segment.parse::<u32>()).collect();
    if parts.iter().any(|part| part.is_err()) {
        return None;
    }
    Some(parts.into_iter().map(|part| part.unwrap()).collect())
}

fn compare_versions(left: &[u32], right: &[u32]) -> i32 {
    let max_len = left.len().max(right.len());
    for idx in 0..max_len {
        let left_value = *left.get(idx).unwrap_or(&0);
        let right_value = *right.get(idx).unwrap_or(&0);
        match left_value.cmp(&right_value) {
            std::cmp::Ordering::Less => return -1,
            std::cmp::Ordering::Greater => return 1,
            std::cmp::Ordering::Equal => {}
        }
    }
    0
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
    fn test_parse_version_parts() {
        assert_eq!(parse_version_parts("1.2.3"), Some(vec![1, 2, 3]));
        assert_eq!(parse_version_parts("1.0"), Some(vec![1, 0]));
        assert_eq!(parse_version_parts("10.20.30"), Some(vec![10, 20, 30]));
        assert_eq!(parse_version_parts("invalid"), None);
        assert_eq!(parse_version_parts("1.a.2"), None);
    }

    #[test]
    fn test_compare_versions() {
        // Equal
        assert_eq!(compare_versions(&[1, 2, 3], &[1, 2, 3]), 0);
        // Left greater
        assert_eq!(compare_versions(&[1, 2, 4], &[1, 2, 3]), 1);
        assert_eq!(compare_versions(&[1, 3, 0], &[1, 2, 3]), 1);
        assert_eq!(compare_versions(&[2, 0, 0], &[1, 2, 3]), 1);
        assert_eq!(compare_versions(&[1, 2, 3, 1], &[1, 2, 3]), 1);
        // Left smaller
        assert_eq!(compare_versions(&[1, 2, 2], &[1, 2, 3]), -1);
        assert_eq!(compare_versions(&[1, 1, 9], &[1, 2, 3]), -1);
        assert_eq!(compare_versions(&[0, 9, 9], &[1, 2, 3]), -1);
        assert_eq!(compare_versions(&[1, 2], &[1, 2, 3]), -1);
    }

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
    fn test_check_version_file_workspace_older_is_ok() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child(".jlo-version").write_str("0.9.0").unwrap();
        let mut diagnostics = Diagnostics::default();
        check_version_file(temp.path(), "1.0.0", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 0);
    }

    #[test]
    fn test_check_version_file_workspace_newer_is_error() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child(".jlo-version").write_str("2.0.0").unwrap();
        let mut diagnostics = Diagnostics::default();
        check_version_file(temp.path(), "1.0.0", &mut diagnostics);
        assert_eq!(diagnostics.error_count(), 1);
        assert!(
            diagnostics.errors()[0].message.contains("Workspace version is newer than the binary")
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

    fn create_valid_workspace(temp: &assert_fs::TempDir) {
        temp.child(".jules/JULES.md").touch().unwrap();
        temp.child(".jules/README.md").touch().unwrap();
        temp.child(".jlo/config.toml").touch().unwrap();
        temp.child(".jules/.jlo-version").write_str(env!("CARGO_PKG_VERSION")).unwrap();

        // Layers
        for layer in Layer::ALL {
            // Runtime artifacts (contracts, schemas, prompts) in .jules/roles
            let jules_layer_dir = temp.child(format!(".jules/roles/{}", layer.dir_name()));
            jules_layer_dir.create_dir_all().unwrap();
            if layer != Layer::Implementers {
                jules_layer_dir.child("schemas").create_dir_all().unwrap();
            }
            jules_layer_dir.child("tasks").create_dir_all().unwrap();
            jules_layer_dir.child(layer.prompt_template_name()).touch().unwrap();

            jules_layer_dir.child("contracts.yml").touch().unwrap();

            if layer.is_single_role() {
                if layer == Layer::Narrators {
                    jules_layer_dir.child("schemas/changes.yml").touch().unwrap();
                }
            } else {
                // Multi-role layers have role definitions in .jlo/roles
                let jlo_role_dir = temp.child(format!(".jlo/roles/{}/my-role", layer.dir_name()));
                jlo_role_dir.create_dir_all().unwrap();
                jlo_role_dir.child("role.yml").touch().unwrap();
            }
        }

        // Root schedule in .jlo/
        temp.child(".jlo/scheduled.toml").touch().unwrap();

        // Flat exchange structure in .jules/
        let exchange = temp.child(".jules/exchange");
        exchange.child("events").create_dir_all().unwrap();
        exchange.child("issues").create_dir_all().unwrap();

        // We need to match inputs for event states and issue labels
        exchange.child("events/pending").create_dir_all().unwrap();
        exchange.child("issues/tests").create_dir_all().unwrap();

        // Innovator rooms directory
        exchange.child("innovators").create_dir_all().unwrap();

        temp.child(".jules/workstations").create_dir_all().unwrap();
    }

    #[test]
    fn test_structural_checks_success() {
        let temp = assert_fs::TempDir::new().unwrap();
        create_valid_workspace(&temp);

        let mut diagnostics = Diagnostics::default();
        let issue_labels = vec!["tests".to_string()];
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            issue_labels: &issue_labels,
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
        create_valid_workspace(&temp);

        // Remove config.toml
        std::fs::remove_file(temp.path().join(".jlo/config.toml")).unwrap();
        // Remove JULES.md
        std::fs::remove_file(temp.path().join(".jules/JULES.md")).unwrap();

        let mut diagnostics = Diagnostics::default();
        let issue_labels = vec!["tests".to_string()];
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            issue_labels: &issue_labels,
            event_states: &event_states,
        };

        structural_checks(inputs, &mut diagnostics);

        // Expect at least 2 errors
        assert!(diagnostics.error_count() >= 2);
        let errors: Vec<String> = diagnostics.errors().iter().map(|e| e.message.clone()).collect();
        // The error message is "Missing required file" based on ensure_path_exists
        assert!(errors.contains(&"Missing required file".to_string()));
    }

    #[test]
    fn test_structural_checks_missing_layer_files() {
        let temp = assert_fs::TempDir::new().unwrap();
        create_valid_workspace(&temp);

        // Remove implementers contracts
        std::fs::remove_file(temp.path().join(".jules/roles/implementers/contracts.yml")).unwrap();

        let mut diagnostics = Diagnostics::default();
        let issue_labels = vec!["tests".to_string()];
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            issue_labels: &issue_labels,
            event_states: &event_states,
        };

        structural_checks(inputs, &mut diagnostics);

        assert!(diagnostics.error_count() >= 1);
        let errors: Vec<String> = diagnostics.errors().iter().map(|e| e.message.clone()).collect();
        assert!(errors.contains(&"Missing contracts.yml".to_string()));
    }
}
