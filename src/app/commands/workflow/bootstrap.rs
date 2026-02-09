//! Workflow bootstrap: deterministic projection of `.jules/` from `.jlo/` + scaffold.
//!
//! Runs as the first step in workflow execution, guaranteeing that the runtime
//! workspace matches the control-plane intent before any agent job.
//!
//! Invariants:
//! - Missing `.jlo/` is a hard failure.
//! - Missing `.jlo/.jlo-version` is a hard failure.
//! - Managed framework files are always materialized from the embedded scaffold.
//! - Mutable control inputs from `.jlo/` are overlaid onto `.jules/`.
//! - Workstreams absent from `.jlo/workstreams/` are deleted from `.jules/workstreams/`.
//! - Roles absent from `.jlo/roles/` are deleted from `.jules/roles/`.
//! - Identical inputs produce no filesystem diff (idempotent).

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::workspace::manifest::{MANIFEST_FILENAME, hash_content, is_default_role_file};
use crate::domain::workspace::workspace_layout::{JLO_DIR, JULES_DIR, VERSION_FILE};
use crate::domain::{AppError, Layer, ScaffoldManifest};
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Options for the bootstrap command.
#[derive(Debug)]
pub struct WorkflowBootstrapOptions {
    /// Root path of the workspace (on the `jules` branch).
    pub root: PathBuf,
}

/// Output of the bootstrap command.
#[derive(Debug, Serialize)]
pub struct WorkflowBootstrapOutput {
    /// Whether materialization was performed.
    pub materialized: bool,
    /// The jlo version used for materialization.
    pub version: String,
    /// Number of files written during materialization.
    pub files_written: usize,
}

/// Execute the workflow bootstrap.
///
/// Deterministically projects `.jules/` from `.jlo/` control inputs and embedded scaffold.
pub fn execute(
    store: &impl WorkspaceStore,
    templates: &impl RoleTemplateStore,
    _options: WorkflowBootstrapOptions,
) -> Result<WorkflowBootstrapOutput, AppError> {
    let current_version = env!("CARGO_PKG_VERSION");

    // --- Hard preconditions ---
    if !store.jlo_exists() {
        return Err(AppError::Validation(
            "Bootstrap requires .jlo/ control plane. Run 'jlo init' on your control branch first."
                .to_string(),
        ));
    }

    let version_file_path = format!("{}/{}", JLO_DIR, VERSION_FILE);
    if !store.file_exists(&version_file_path) {
        return Err(AppError::WorkspaceIntegrity(
            "Missing .jlo/.jlo-version. Control plane is incomplete.".to_string(),
        ));
    }

    let files_written = project_runtime(store, templates, current_version)?;

    Ok(WorkflowBootstrapOutput {
        materialized: true,
        version: current_version.to_string(),
        files_written,
    })
}

/// Full deterministic projection of `.jules/` from scaffold + `.jlo/` overlay.
fn project_runtime(
    store: &impl WorkspaceStore,
    templates: &impl RoleTemplateStore,
    version: &str,
) -> Result<usize, AppError> {
    // Counts write operations performed; overlay may overwrite scaffold files,
    // so this can exceed the unique file count in the final projection.
    let mut files_written: usize = 0;

    // 1. Materialize managed framework files from embedded scaffold
    let scaffold_files = templates.scaffold_files();
    store.create_structure(&scaffold_files)?;
    store.write_version(version)?;
    files_written += scaffold_files.len() + 1; // +1 for version

    // 2. Overlay mutable control inputs from .jlo/ onto .jules/
    files_written += overlay_control_inputs(store)?;

    // 3. Delete projected workstreams absent from .jlo/
    delete_absent_workstreams(store)?;

    // 4. Delete projected roles absent from .jlo/
    delete_absent_roles(store)?;

    // 5. Write managed manifest
    let mut map = BTreeMap::new();
    for file in &scaffold_files {
        if is_default_role_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    let managed_manifest = ScaffoldManifest::from_map(map);
    let manifest_content = managed_manifest.to_yaml()?;
    let manifest_path = format!("{}/{}", JULES_DIR, MANIFEST_FILENAME);
    store.write_file(&manifest_path, &manifest_content)?;
    files_written += 1;

    Ok(files_written)
}

/// Copy mutable control inputs from `.jlo/` to their `.jules/` counterparts.
fn overlay_control_inputs(store: &impl WorkspaceStore) -> Result<usize, AppError> {
    let jlo_path = Path::new(JLO_DIR);
    let mut count = 0;

    // Collect all files under .jlo/ recursively and project only allowlisted inputs to .jules/
    let files = collect_files_recursive(store, jlo_path, jlo_path)?;
    for (rel_path, content) in files {
        if !should_project_control_file(&rel_path) {
            continue;
        }

        let jules_rel = format!("{}/{}", JULES_DIR, rel_path);
        store.write_file(&jules_rel, &content)?;
        count += 1;
    }

    Ok(count)
}

/// Delete workstreams in `.jules/workstreams/` that are absent from `.jlo/workstreams/`.
fn delete_absent_workstreams(store: &impl WorkspaceStore) -> Result<(), AppError> {
    let jlo_ws_rel = format!("{}/workstreams", JLO_DIR);
    let jules_ws_rel = format!("{}/workstreams", JULES_DIR);

    let jlo_workstreams = list_workstreams_with_schedule(store, Path::new(&jlo_ws_rel))?;
    let jules_workstreams = list_subdirs(store, Path::new(&jules_ws_rel))?;

    for ws in &jules_workstreams {
        if !jlo_workstreams.contains(ws) {
            let ws_path = format!("{}/{}", jules_ws_rel, ws);
            store.remove_dir_all(&ws_path)?;
        }
    }

    Ok(())
}

/// Delete roles in `.jules/roles/<layer>/roles/` that are absent from `.jlo/roles/<layer>/roles/`.
fn delete_absent_roles(store: &impl WorkspaceStore) -> Result<(), AppError> {
    for layer in Layer::ALL {
        if layer.is_single_role() {
            continue;
        }

        let jlo_roles_rel = format!("{}/roles/{}/roles", JLO_DIR, layer.dir_name());
        let jules_roles_rel = format!("{}/roles/{}/roles", JULES_DIR, layer.dir_name());

        let jlo_roles = list_roles_with_definition(store, Path::new(&jlo_roles_rel))?;
        let jules_roles = list_subdirs(store, Path::new(&jules_roles_rel))?;

        for role in &jules_roles {
            if !jlo_roles.contains(role) {
                let role_path = format!("{}/{}", jules_roles_rel, role);
                store.remove_dir_all(&role_path)?;
            }
        }
    }

    Ok(())
}

fn should_project_control_file(rel_path: &str) -> bool {
    if rel_path == VERSION_FILE {
        return false;
    }

    if rel_path == "config.toml" {
        return true;
    }

    if rel_path.starts_with("setup/") {
        return true;
    }

    let path = Path::new(rel_path);
    let components: Vec<_> =
        path.components().map(|c| c.as_os_str().to_str().unwrap_or("")).collect();
    if components.len() == 5
        && components[0] == "roles"
        && components[2] == "roles"
        && components[4] == "role.yml"
    {
        return true;
    }

    if components.len() == 3 && components[0] == "workstreams" && components[2] == "scheduled.toml"
    {
        return true;
    }

    false
}

/// Recursively collect all files under a directory as (relative_path, content) pairs.
fn collect_files_recursive(
    store: &impl WorkspaceStore,
    base: &Path,
    dir: &Path,
) -> Result<Vec<(String, String)>, AppError> {
    let mut result = Vec::new();
    let dir_str = dir.to_str().ok_or_else(|| {
        AppError::InternalError(format!("Invalid path encoding: {}", dir.display()))
    })?;

    if !store.is_dir(dir_str) {
        return Ok(result);
    }

    let entries = store.list_dir(dir_str)?;
    // entries are full paths (or at least paths that store returns).
    // We need to extract filename to append to current `dir` (which is relative to root).

    let mut sorted_entries = entries;
    sorted_entries.sort();

    for entry_path in sorted_entries {
        let file_name = entry_path.file_name().ok_or_else(|| {
            AppError::InternalError("Failed to extract filename".to_string())
        })?;
        let rel_path = dir.join(file_name);
        let rel_path_str = rel_path.to_str().ok_or_else(|| {
            AppError::InternalError(format!("Invalid path encoding: {}", rel_path.display()))
        })?;

        if store.is_symlink(rel_path_str) {
            continue;
        }

        if store.is_dir(rel_path_str) {
            result.extend(collect_files_recursive(store, base, &rel_path)?);
        } else if store.file_exists(rel_path_str) {
            let rel_to_base = rel_path.strip_prefix(base).map_err(|e| {
                AppError::InternalError(format!("Path strip failed: {}", e))
            })?;
            let content = store.read_file(rel_path_str)?;
            result.push((rel_to_base.to_string_lossy().to_string(), content));
        }
    }

    Ok(result)
}

/// List subdirectory names (not paths) for a directory.
fn list_subdirs(store: &impl WorkspaceStore, dir: &Path) -> Result<BTreeSet<String>, AppError> {
    let mut names = BTreeSet::new();
    let dir_str = dir.to_str().unwrap_or_default();

    if !store.is_dir(dir_str) {
        return Ok(names);
    }

    let entries = store.list_dir(dir_str)?;
    for entry in entries {
         let file_name = entry.file_name().unwrap();
         // Construct path relative to root to check is_dir
         let path = dir.join(file_name);
         if store.is_dir(path.to_str().unwrap()) {
             names.insert(file_name.to_string_lossy().to_string());
         }
    }
    Ok(names)
}

fn list_workstreams_with_schedule(
    store: &impl WorkspaceStore,
    dir: &Path,
) -> Result<BTreeSet<String>, AppError> {
    let mut names = BTreeSet::new();
    let dir_str = dir.to_str().unwrap_or_default();

    if !store.is_dir(dir_str) {
        return Ok(names);
    }

    let entries = store.list_dir(dir_str)?;
    for entry in entries {
        let file_name = entry.file_name().unwrap();
        let path = dir.join(file_name);
        if store.is_dir(path.to_str().unwrap()) {
            let schedule = path.join("scheduled.toml");
            if store.file_exists(schedule.to_str().unwrap()) {
                names.insert(file_name.to_string_lossy().to_string());
            }
        }
    }
    Ok(names)
}

fn list_roles_with_definition(
    store: &impl WorkspaceStore,
    dir: &Path,
) -> Result<BTreeSet<String>, AppError> {
    let mut names = BTreeSet::new();
    let dir_str = dir.to_str().unwrap_or_default();

    if !store.is_dir(dir_str) {
        return Ok(names);
    }

    let entries = store.list_dir(dir_str)?;
    for entry in entries {
        let file_name = entry.file_name().unwrap();
        let path = dir.join(file_name);
        if store.is_dir(path.to_str().unwrap()) {
            let role_file = path.join("role.yml");
            if store.file_exists(role_file.to_str().unwrap()) {
                names.insert(file_name.to_string_lossy().to_string());
            }
        }
    }
    Ok(names)
}
