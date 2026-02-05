use std::fs;
use std::path::{Path, PathBuf};

use crate::app::commands::run::parse_config_content;
use crate::domain::{AppError, Layer, RunConfig};
use crate::services::assets::scaffold_assets::scaffold_file_content;
use crate::services::assets::workstream_template_assets::workstream_template_content;

use super::DoctorOptions;
use super::diagnostics::Diagnostics;

pub fn collect_workstreams(
    jules_path: &Path,
    filter: Option<&str>,
) -> Result<Vec<String>, AppError> {
    let workstreams_dir = jules_path.join("workstreams");
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

pub fn read_run_config(
    jules_path: &Path,
    diagnostics: &mut Diagnostics,
) -> Result<RunConfig, AppError> {
    let config_path = jules_path.join("config.toml");
    if !config_path.exists() {
        diagnostics.push_error(config_path.display().to_string(), "Missing .jules/config.toml");
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
    pub options: &'a DoctorOptions,
    pub applied_fixes: &'a mut Vec<String>,
}

pub fn structural_checks(inputs: StructuralInputs<'_>, diagnostics: &mut Diagnostics) {
    ensure_path_exists(
        inputs.root,
        ".jules/JULES.md",
        inputs.options,
        inputs.applied_fixes,
        diagnostics,
        true,
    );
    ensure_path_exists(
        inputs.root,
        ".jules/README.md",
        inputs.options,
        inputs.applied_fixes,
        diagnostics,
        true,
    );
    ensure_path_exists(
        inputs.root,
        ".jules/config.toml",
        inputs.options,
        inputs.applied_fixes,
        diagnostics,
        true,
    );
    ensure_path_exists(
        inputs.root,
        ".jules/.jlo-version",
        inputs.options,
        inputs.applied_fixes,
        diagnostics,
        true,
    );
    ensure_directory_exists(
        inputs.jules_path.join("roles"),
        inputs.options,
        inputs.applied_fixes,
        diagnostics,
    );
    ensure_directory_exists(
        inputs.jules_path.join("workstreams"),
        inputs.options,
        inputs.applied_fixes,
        diagnostics,
    );
    // Narrator output directory
    ensure_directory_exists(
        inputs.jules_path.join("changes"),
        inputs.options,
        inputs.applied_fixes,
        diagnostics,
    );

    check_version_file(inputs.jules_path, env!("CARGO_PKG_VERSION"), diagnostics);

    for layer in Layer::ALL {
        let layer_dir = inputs.jules_path.join("roles").join(layer.dir_name());
        if !layer_dir.exists() {
            diagnostics.push_error(layer_dir.display().to_string(), "Missing layer directory");
            continue;
        }

        let contracts = layer_dir.join("contracts.yml");
        if !contracts.exists() {
            diagnostics.push_error(contracts.display().to_string(), "Missing contracts.yml");
        }

        // Check schemas/ directory (all layers have this)
        let schemas_dir = layer_dir.join("schemas");
        if !schemas_dir.exists() {
            diagnostics.push_error(schemas_dir.display().to_string(), "Missing schemas/");
        }

        // Check prompt_assembly.yml (all layers have this)
        let prompt_assembly = layer_dir.join("prompt_assembly.yml");
        if !prompt_assembly.exists() {
            diagnostics
                .push_error(prompt_assembly.display().to_string(), "Missing prompt_assembly.yml");
        }

        if layer.is_single_role() {
            let prompt = layer_dir.join("prompt.yml");
            if !prompt.exists() {
                diagnostics.push_error(prompt.display().to_string(), "Missing prompt.yml");
            }

            // Narrator requires change.yml schema template
            if layer == Layer::Narrators {
                let change_template = layer_dir.join("schemas").join("change.yml");
                if !change_template.exists() {
                    diagnostics
                        .push_error(change_template.display().to_string(), "Missing change.yml");
                }
            }
        } else {
            // Check for prompt.yml in multi-role layers
            let prompt = layer_dir.join("prompt.yml");
            if !prompt.exists() {
                diagnostics.push_error(prompt.display().to_string(), "Missing prompt.yml");
            }

            // Check roles/ container directory for multi-role layers
            let roles_container = layer_dir.join("roles");
            if !roles_container.exists() {
                diagnostics
                    .push_error(roles_container.display().to_string(), "Missing roles/ directory");
            } else {
                for entry in list_subdirs(&roles_container, diagnostics) {
                    let role_file = entry.join("role.yml");
                    if !role_file.exists() {
                        diagnostics.push_error(role_file.display().to_string(), "Missing role.yml");
                    }
                }
            }
        }
    }

    for workstream in inputs.workstreams {
        let ws_dir = inputs.jules_path.join("workstreams").join(workstream);
        if !ws_dir.exists() {
            diagnostics.push_error(ws_dir.display().to_string(), "Missing workstream directory");
            continue;
        }

        let scheduled_path = ws_dir.join("scheduled.toml");
        ensure_workstream_template_exists(
            scheduled_path,
            "scheduled.toml",
            inputs.options,
            inputs.applied_fixes,
            diagnostics,
        );

        // Exchange directory structure (events and issues)
        let exchange_dir = ws_dir.join("exchange");
        ensure_directory_exists(
            exchange_dir.clone(),
            inputs.options,
            inputs.applied_fixes,
            diagnostics,
        );

        let events_dir = exchange_dir.join("events");
        ensure_directory_exists(
            events_dir.clone(),
            inputs.options,
            inputs.applied_fixes,
            diagnostics,
        );
        for state in inputs.event_states {
            ensure_directory_exists(
                events_dir.join(state),
                inputs.options,
                inputs.applied_fixes,
                diagnostics,
            );
        }

        let issues_dir = exchange_dir.join("issues");
        ensure_directory_exists(
            issues_dir.clone(),
            inputs.options,
            inputs.applied_fixes,
            diagnostics,
        );
        for label in inputs.issue_labels {
            ensure_directory_exists(
                issues_dir.join(label),
                inputs.options,
                inputs.applied_fixes,
                diagnostics,
            );
        }

        // Workstations directory
        let workstations_dir = ws_dir.join("workstations");
        ensure_directory_exists(
            workstations_dir,
            inputs.options,
            inputs.applied_fixes,
            diagnostics,
        );
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

fn ensure_directory_exists(
    path: PathBuf,
    options: &DoctorOptions,
    applied_fixes: &mut Vec<String>,
    diagnostics: &mut Diagnostics,
) {
    if path.exists() {
        return;
    }

    if options.fix {
        if let Err(err) = fs::create_dir_all(&path) {
            diagnostics.push_error(path.display().to_string(), err.to_string());
        } else {
            applied_fixes.push(format!("Created directory {}", path.display()));
            diagnostics.push_warning(path.display().to_string(), "Created missing directory");
        }
    } else {
        diagnostics.push_error(path.display().to_string(), "Missing directory");
    }
}

fn ensure_path_exists(
    root: &Path,
    rel_path: &str,
    options: &DoctorOptions,
    applied_fixes: &mut Vec<String>,
    diagnostics: &mut Diagnostics,
    fixable_from_scaffold: bool,
) {
    let full_path = root.join(rel_path);
    if full_path.exists() {
        return;
    }

    if options.fix && fixable_from_scaffold {
        attempt_fix_file(full_path, rel_path, options, applied_fixes, diagnostics);
    } else {
        diagnostics.push_error(full_path.display().to_string(), "Missing required file");
    }
}

fn attempt_fix_file(
    full_path: PathBuf,
    scaffold_path: &str,
    options: &DoctorOptions,
    applied_fixes: &mut Vec<String>,
    diagnostics: &mut Diagnostics,
) {
    if !options.fix {
        diagnostics.push_error(full_path.display().to_string(), "Missing required file");
        return;
    }

    let content = match scaffold_file_content(scaffold_path) {
        Some(content) => content,
        None => {
            diagnostics.push_error(
                full_path.display().to_string(),
                "Missing required file (no scaffold fix available)",
            );
            return;
        }
    };

    if let Some(parent) = full_path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        diagnostics.push_error(full_path.display().to_string(), err.to_string());
        return;
    }

    if let Err(err) = fs::write(&full_path, content) {
        diagnostics.push_error(full_path.display().to_string(), err.to_string());
        return;
    }

    applied_fixes.push(format!("Restored {}", full_path.display()));
    diagnostics
        .push_warning(full_path.display().to_string(), "Restored missing file from scaffold");
}

fn ensure_workstream_template_exists(
    full_path: PathBuf,
    template_path: &str,
    options: &DoctorOptions,
    applied_fixes: &mut Vec<String>,
    diagnostics: &mut Diagnostics,
) {
    if full_path.exists() {
        return;
    }

    if !options.fix {
        diagnostics
            .push_error(full_path.display().to_string(), format!("Missing {}", template_path));
        return;
    }

    let content = match workstream_template_content(template_path) {
        Ok(content) => content,
        Err(err) => {
            diagnostics.push_error(full_path.display().to_string(), err.to_string());
            return;
        }
    };

    if let Some(parent) = full_path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        diagnostics.push_error(full_path.display().to_string(), err.to_string());
        return;
    }

    if let Err(err) = fs::write(&full_path, content) {
        diagnostics.push_error(full_path.display().to_string(), err.to_string());
        return;
    }

    applied_fixes.push(format!("Restored {}", full_path.display()));
    diagnostics
        .push_warning(full_path.display().to_string(), "Restored missing file from templates");
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
        let ws_dir = temp.child("workstreams");
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

    #[test]
    fn test_structural_checks_clean() {
        let temp = assert_fs::TempDir::new().unwrap();
        let jules_path = temp.child(".jules");
        jules_path.create_dir_all().unwrap();

        // Create critical files
        temp.child(".jules/JULES.md").touch().unwrap();
        temp.child(".jules/README.md").touch().unwrap();
        temp.child(".jules/config.toml").touch().unwrap();
        temp.child(".jules/.jlo-version").write_str(env!("CARGO_PKG_VERSION")).unwrap();

        // Create directories
        jules_path.child("roles").create_dir_all().unwrap();
        jules_path.child("workstreams").create_dir_all().unwrap();
        jules_path.child("changes").create_dir_all().unwrap();

        // Mock layer roles
        for layer in Layer::ALL {
            let layer_dir = jules_path.child("roles").child(layer.dir_name());
            layer_dir.create_dir_all().unwrap();
            layer_dir.child("contracts.yml").touch().unwrap();
            layer_dir.child("schemas").create_dir_all().unwrap();
            layer_dir.child("prompt_assembly.yml").touch().unwrap();
            layer_dir.child("prompt.yml").touch().unwrap();

            if layer == Layer::Narrators {
                layer_dir.child("schemas/change.yml").touch().unwrap();
            }
            if !layer.is_single_role() {
                layer_dir.child("roles").create_dir_all().unwrap();
            }
        }

        let mut diagnostics = Diagnostics::default();
        let options = DoctorOptions::default();
        let mut applied_fixes = Vec::new();
        let workstreams = Vec::new();
        let issue_labels = Vec::new();
        let event_states = Vec::new();

        structural_checks(
            StructuralInputs {
                jules_path: jules_path.path(),
                root: temp.path(),
                workstreams: &workstreams,
                issue_labels: &issue_labels,
                event_states: &event_states,
                options: &options,
                applied_fixes: &mut applied_fixes,
            },
            &mut diagnostics,
        );

        assert_eq!(diagnostics.error_count(), 0, "Errors found: {:?}", diagnostics.errors());
    }

    #[test]
    fn test_structural_checks_missing_critical_files() {
        let temp = assert_fs::TempDir::new().unwrap();
        let jules_path = temp.child(".jules");
        jules_path.create_dir_all().unwrap();

        // Environment is empty except for .jules directory

        let mut diagnostics = Diagnostics::default();
        let options = DoctorOptions::default();
        let mut applied_fixes = Vec::new();
        let workstreams = Vec::new();
        let issue_labels = Vec::new();
        let event_states = Vec::new();

        structural_checks(
            StructuralInputs {
                jules_path: jules_path.path(),
                root: temp.path(),
                workstreams: &workstreams,
                issue_labels: &issue_labels,
                event_states: &event_states,
                options: &options,
                applied_fixes: &mut applied_fixes,
            },
            &mut diagnostics,
        );

        assert!(diagnostics.error_count() > 0);
        let errors = diagnostics.errors();
        assert!(errors.iter().any(|e| e.file.contains("JULES.md")));
        assert!(errors.iter().any(|e| e.file.contains("config.toml")));
    }

    #[test]
    fn test_structural_checks_fix() {
        let temp = assert_fs::TempDir::new().unwrap();
        let jules_path = temp.child(".jules");
        jules_path.create_dir_all().unwrap();

        let mut diagnostics = Diagnostics::default();
        let options = DoctorOptions { fix: true, ..Default::default() };
        let mut applied_fixes = Vec::new();
        let workstreams = Vec::new();
        let issue_labels = Vec::new();
        let event_states = Vec::new();

        structural_checks(
            StructuralInputs {
                jules_path: jules_path.path(),
                root: temp.path(),
                workstreams: &workstreams,
                issue_labels: &issue_labels,
                event_states: &event_states,
                options: &options,
                applied_fixes: &mut applied_fixes,
            },
            &mut diagnostics,
        );

        // Files should be created.
        // Note: verifying content restore works requires valid scaffold assets.
        // Assuming unit tests run in environment where assets are linked.
        assert!(temp.child(".jules/JULES.md").exists());
        assert!(temp.child(".jules/config.toml").exists());

        // We expect warnings about restored files
        assert!(diagnostics.warning_count() > 0);
        assert!(!applied_fixes.is_empty());
    }
}
