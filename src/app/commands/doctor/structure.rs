use std::fs;
use std::path::{Path, PathBuf};

use crate::app::commands::run::parse_config_content;
use crate::domain::{AppError, Layer, RunConfig};

use super::diagnostics::Diagnostics;

pub fn collect_workstreams(root: &Path, filter: Option<&str>) -> Result<Vec<String>, AppError> {
    let workstreams_dir = root.join(".jlo/workstreams");
    if !workstreams_dir.exists() {
        return Ok(Vec::new());
    }

    let mut workstreams = Vec::new();
    for entry in fs::read_dir(&workstreams_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            workstreams.push(name);
        }
    }
    workstreams.sort();

    if let Some(target) = filter {
        if !workstreams.contains(&target.to_string()) {
            return Err(AppError::Validation(format!("Workstream '{}' not found", target)));
        }
        return Ok(vec![target.to_string()]);
    }

    Ok(workstreams)
}

pub fn read_run_config(root: &Path, diagnostics: &mut Diagnostics) -> Result<RunConfig, AppError> {
    let config_path = root.join(".jlo/config.toml");
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
    pub workstreams: &'a [String],
    pub issue_labels: &'a [String],
    pub event_states: &'a [String],
}

pub fn structural_checks(inputs: StructuralInputs<'_>, diagnostics: &mut Diagnostics) {
    ensure_path_exists(inputs.root, ".jules/JULES.md", diagnostics);
    ensure_path_exists(inputs.root, ".jules/README.md", diagnostics);
    ensure_path_exists(inputs.root, ".jlo/config.toml", diagnostics);
    ensure_path_exists(inputs.root, ".jules/.jlo-version", diagnostics);
    ensure_directory_exists(inputs.jules_path.join("roles"), diagnostics);
    // Workstreams dir in .jules might not exist if no runtime artifacts yet?
    // Actually bootstrap scaffold creates .jules/workstreams/generic...
    // But we should probably check .jlo/workstreams too.
    ensure_directory_exists(inputs.root.join(".jlo/roles"), diagnostics);
    ensure_directory_exists(inputs.root.join(".jlo/workstreams"), diagnostics);
    // Narrator output directory
    ensure_directory_exists(inputs.jules_path.join("changes"), diagnostics);

    check_version_file(inputs.jules_path, env!("CARGO_PKG_VERSION"), diagnostics);

    for layer in Layer::ALL {
        let layer_dir = inputs.jules_path.join("roles").join(layer.dir_name());
        if !layer_dir.exists() {
            diagnostics.push_error(layer_dir.display().to_string(), "Missing layer directory");
            continue;
        }

        // Innovators use phase-specific contracts; all other layers use contracts.yml
        if layer == Layer::Innovators {
            for phase in ["creation", "refinement"] {
                let contract_file = layer_dir.join(format!("contracts_{}.yml", phase));
                if !contract_file.exists() {
                    diagnostics.push_error(
                        contract_file.display().to_string(),
                        format!("Missing contracts_{}.yml", phase),
                    );
                }
            }
        } else {
            let contracts = layer_dir.join("contracts.yml");
            if !contracts.exists() {
                diagnostics.push_error(contracts.display().to_string(), "Missing contracts.yml");
            }
        }

        // Check schemas/ directory (all layers have this)
        let schemas_dir = layer_dir.join("schemas");
        if !schemas_dir.exists() {
            diagnostics.push_error(schemas_dir.display().to_string(), "Missing schemas/");
        }

        // Check prompt_assembly.j2 (all layers have this)
        let prompt_assembly = layer_dir.join("prompt_assembly.j2");
        if !prompt_assembly.exists() {
            diagnostics
                .push_error(prompt_assembly.display().to_string(), "Missing prompt_assembly.j2");
        }

        if layer.is_single_role() {
            // Narrator requires change.yml schema template
            if layer == Layer::Narrators {
                let change_template = layer_dir.join("schemas").join("change.yml");
                if !change_template.exists() {
                    diagnostics
                        .push_error(change_template.display().to_string(), "Missing change.yml");
                }
            }
        } else {
            // Check roles/ container directory for multi-role layers in .jlo/
            // The .jules/ structure for multi-role layers (roles/ container) might technically exist from scaffold
            // but the actual role definitions are in .jlo/.
            // structure_checks needs to verify that for every role in .jlo/, it exists.
            let jlo_layer_dir = inputs.root.join(".jlo/roles").join(layer.dir_name());
            let jlo_roles_container = jlo_layer_dir.join("roles");

            if !jlo_roles_container.exists() {
                // It's possible the user hasn't created any roles yet, but the directory should probably exist if init ran?
                // Actually init scaffolds .jlo/roles/<layer>/roles/.
                diagnostics.push_error(
                    jlo_roles_container.display().to_string(),
                    "Missing .jlo roles/ directory",
                );
            } else {
                for entry in list_subdirs(&jlo_roles_container, diagnostics) {
                    let role_file = entry.join("role.yml");
                    if !role_file.exists() {
                        diagnostics.push_error(role_file.display().to_string(), "Missing role.yml");
                    }
                }
            }
        }
    }

    for workstream in inputs.workstreams {
        let jlo_ws_dir = inputs.root.join(".jlo/workstreams").join(workstream);
        let jules_ws_dir = inputs.jules_path.join("workstreams").join(workstream);

        if !jlo_ws_dir.exists() {
            diagnostics
                .push_error(jlo_ws_dir.display().to_string(), "Missing .jlo workstream definition");
            // If it's missing in .jlo, we can't really check much else, but we should continue?
            // But if collect_workstreams scanned .jlo, it should exist.
        } else {
            let scheduled_path = jlo_ws_dir.join("scheduled.toml");
            ensure_workstream_template_exists(scheduled_path, "scheduled.toml", diagnostics);
        }

        // We assume jules_ws_dir should exist?
        // If it's a new workstream, it might not exist in .jules yet.
        // But doctor checks structure.
        // If the user wants to run it, it should exist?
        // Actually, bootstrap NO LONGER projects it.
        // So jules_ws_dir might NOT exist.
        // But if it DOES exist, we should check its structure?
        // Or should we mandate it exists?
        // Events and issues are in .jules/workstreams/<ws>/...
        // If the directory is missing, then no events/issues can exist.
        // Getting "Missing directory" errors for a fresh workstream in .jules might be annoying if it's auto-created on first run.
        // But previously bootstrap created it. Now it doesn't.
        // So we should probably NOT error if jules_ws_dir doesn't exist?
        // BUT, the test `test_api_coverage_full_flow` calls `workflow_bootstrap_at` and then asserts `.jules` exists.
        // And then runs doctor.
        // If doctor fails because jules_ws_dir is missing, that's what we see.
        // Maybe ensure_directory_exists should be conditional?

        // For now, let's skip checking .jules structure for the workstream if the workstream dir doesn't exist in .jules.
        if !jules_ws_dir.exists() {
            continue;
        }

        // Exchange directory structure (events and issues)
        let exchange_dir = jules_ws_dir.join("exchange");
        ensure_directory_exists(exchange_dir.clone(), diagnostics);

        let events_dir = exchange_dir.join("events");
        ensure_directory_exists(events_dir.clone(), diagnostics);
        for state in inputs.event_states {
            ensure_directory_exists(events_dir.join(state), diagnostics);
        }

        let issues_dir = exchange_dir.join("issues");
        ensure_directory_exists(issues_dir.clone(), diagnostics);
        for label in inputs.issue_labels {
            ensure_directory_exists(issues_dir.join(label), diagnostics);
        }

        // Workstations directory
        let workstations_dir = jules_ws_dir.join("workstations");
        ensure_directory_exists(workstations_dir, diagnostics);

        // Innovator rooms directory
        let innovators_dir = exchange_dir.join("innovators");
        ensure_directory_exists(innovators_dir.clone(), diagnostics);

        // Validate each innovator room structure
        if innovators_dir.exists() {
            for persona_dir in list_subdirs(&innovators_dir, diagnostics) {
                let comments_dir = persona_dir.join("comments");
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
    let version_path = jules_path.join(".jlo-version");
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

fn ensure_path_exists(root: &Path, rel_path: &str, diagnostics: &mut Diagnostics) {
    let full_path = root.join(rel_path);
    if !full_path.exists() {
        diagnostics.push_error(full_path.display().to_string(), "Missing required file");
    }
}

fn ensure_workstream_template_exists(
    full_path: PathBuf,
    template_path: &str,
    diagnostics: &mut Diagnostics,
) {
    if !full_path.exists() {
        diagnostics
            .push_error(full_path.display().to_string(), format!("Missing {}", template_path));
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

    #[test]
    fn test_collect_workstreams() {
        let temp = assert_fs::TempDir::new().unwrap();
        let ws_dir = temp.child(".jlo/workstreams");
        ws_dir.create_dir_all().unwrap();

        ws_dir.child("ws1").create_dir_all().unwrap();
        ws_dir.child("ws2").create_dir_all().unwrap();
        ws_dir.child("file.txt").touch().unwrap(); // Should be ignored

        // Test listing all
        let result = collect_workstreams(temp.path(), None).unwrap();
        assert_eq!(result, vec!["ws1", "ws2"]);

        // Test filter exists
        let result = collect_workstreams(temp.path(), Some("ws1")).unwrap();
        assert_eq!(result, vec!["ws1"]);

        // Test filter missing
        let result = collect_workstreams(temp.path(), Some("ws3"));
        assert!(result.is_err());
    }

    fn create_valid_workspace(temp: &assert_fs::TempDir) {
        temp.child(".jules/JULES.md").touch().unwrap();
        temp.child(".jules/README.md").touch().unwrap();
        temp.child(".jlo/config.toml").touch().unwrap();
        temp.child(".jules/.jlo-version").write_str(env!("CARGO_PKG_VERSION")).unwrap();
        temp.child(".jules/changes").create_dir_all().unwrap();

        // Layers
        for layer in Layer::ALL {
            // Runtime artifacts (contracts, schemas, prompts) in .jules/roles
            let jules_layer_dir = temp.child(format!(".jules/roles/{}", layer.dir_name()));
            jules_layer_dir.create_dir_all().unwrap();
            jules_layer_dir.child("schemas").create_dir_all().unwrap();
            jules_layer_dir.child("prompt_assembly.j2").touch().unwrap();

            // Innovators use phase-specific contracts; others use contracts.yml
            if layer == Layer::Innovators {
                jules_layer_dir.child("contracts_creation.yml").touch().unwrap();
                jules_layer_dir.child("contracts_refinement.yml").touch().unwrap();
            } else {
                jules_layer_dir.child("contracts.yml").touch().unwrap();
            }

            if layer.is_single_role() {
                if layer == Layer::Narrators {
                    jules_layer_dir.child("schemas/change.yml").touch().unwrap();
                }
            } else {
                // Multi-role layers have role definitions in .jlo/roles
                let jlo_role_dir =
                    temp.child(format!(".jlo/roles/{}/roles/my-role", layer.dir_name()));
                jlo_role_dir.create_dir_all().unwrap();
                jlo_role_dir.child("role.yml").touch().unwrap();
            }
        }

        // Workstream: definition in .jlo
        let jlo_ws_dir = temp.child(".jlo/workstreams/generic");
        jlo_ws_dir.create_dir_all().unwrap();
        jlo_ws_dir.child("scheduled.toml").touch().unwrap();

        // Workstream: runtime in .jules
        let jules_ws_dir = temp.child(".jules/workstreams/generic");
        jules_ws_dir.create_dir_all().unwrap();

        // Exchange structure in .jules
        let exchange = jules_ws_dir.child("exchange");
        exchange.child("events").create_dir_all().unwrap();
        exchange.child("issues").create_dir_all().unwrap();

        // We need to match inputs for event states and issue labels
        exchange.child("events/pending").create_dir_all().unwrap();
        exchange.child("issues/tests").create_dir_all().unwrap();

        // Innovator rooms directory
        exchange.child("innovators").create_dir_all().unwrap();

        jules_ws_dir.child("workstations").create_dir_all().unwrap();
    }

    #[test]
    fn test_structural_checks_success() {
        let temp = assert_fs::TempDir::new().unwrap();
        create_valid_workspace(&temp);

        let mut diagnostics = Diagnostics::default();
        let workstreams = vec!["generic".to_string()];
        let issue_labels = vec!["tests".to_string()];
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            workstreams: &workstreams,
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
        let workstreams = vec!["generic".to_string()];
        let issue_labels = vec!["tests".to_string()];
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            workstreams: &workstreams,
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
        let workstreams = vec!["generic".to_string()];
        let issue_labels = vec!["tests".to_string()];
        let event_states = vec!["pending".to_string()];

        let inputs = StructuralInputs {
            jules_path: &temp.path().join(".jules"),
            root: temp.path(),
            workstreams: &workstreams,
            issue_labels: &issue_labels,
            event_states: &event_states,
        };

        structural_checks(inputs, &mut diagnostics);

        assert!(diagnostics.error_count() >= 1);
        let errors: Vec<String> = diagnostics.errors().iter().map(|e| e.message.clone()).collect();
        assert!(errors.contains(&"Missing contracts.yml".to_string()));
    }
}
