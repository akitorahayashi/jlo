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
use std::path::Path;

use serde::Serialize;

use crate::adapters::embedded_role_template_store::EmbeddedRoleTemplateStore;
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::app::AppContext;
use crate::domain::workspace::manifest::{MANIFEST_FILENAME, hash_content, is_default_role_file};
use crate::domain::workspace::workspace_layout::{JLO_DIR, JULES_DIR, VERSION_FILE};
use crate::domain::{AppError, Layer, ScaffoldManifest};
use crate::ports::{RoleTemplateStore, WorkspaceStore};

/// Options for the bootstrap command.
#[derive(Debug)]
pub struct WorkflowBootstrapOptions {
    /// Root path of the workspace (on the `jules` branch).
    pub root: std::path::PathBuf,
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
pub fn execute(options: WorkflowBootstrapOptions) -> Result<WorkflowBootstrapOutput, AppError> {
    let current_version = env!("CARGO_PKG_VERSION");
    let root = &options.root;

    let store = FilesystemWorkspaceStore::new(root.clone());
    let templates = EmbeddedRoleTemplateStore::new();

    // --- Hard preconditions ---
    let jlo_path = root.join(JLO_DIR);
    if !jlo_path.exists() {
        return Err(AppError::Validation(
            "Bootstrap requires .jlo/ control plane. Run 'jlo init' on your control branch first."
                .to_string(),
        ));
    }

    let jlo_version_path = jlo_path.join(VERSION_FILE);
    if !jlo_version_path.exists() {
        return Err(AppError::WorkspaceIntegrity(
            "Missing .jlo/.jlo-version. Control plane is incomplete.".to_string(),
        ));
    }

    let ctx = AppContext::new(store, templates);
    let files_written = project_runtime(&ctx, root, current_version)?;

    Ok(WorkflowBootstrapOutput {
        materialized: true,
        version: current_version.to_string(),
        files_written,
    })
}

/// Full deterministic projection of `.jules/` from scaffold + `.jlo/` overlay.
fn project_runtime<W, R>(
    ctx: &AppContext<W, R>,
    root: &Path,
    version: &str,
) -> Result<usize, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    let mut files_written: usize = 0;

    // 1. Materialize managed framework files from embedded scaffold
    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;
    ctx.workspace().write_version(version)?;
    files_written += scaffold_files.len() + 1; // +1 for version

    // 2. Overlay mutable control inputs from .jlo/ onto .jules/
    files_written += overlay_control_inputs(ctx, root)?;

    // 3. Delete projected workstreams absent from .jlo/
    delete_absent_workstreams(root)?;

    // 4. Delete projected roles absent from .jlo/
    delete_absent_roles(root)?;

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
    ctx.workspace().write_file(&manifest_path, &manifest_content)?;
    files_written += 1;

    Ok(files_written)
}

/// Copy mutable control inputs from `.jlo/` to their `.jules/` counterparts.
///
/// Handles:
/// - `.jlo/roles/<layer>/roles/<role>/role.yml` → `.jules/roles/<layer>/roles/<role>/role.yml`
/// - `.jlo/workstreams/<ws>/scheduled.toml` → `.jules/workstreams/<ws>/scheduled.toml`
/// - `.jlo/config.toml` → `.jules/config.toml`
/// - `.jlo/setup/**` → `.jules/setup/**`
fn overlay_control_inputs<W, R>(ctx: &AppContext<W, R>, root: &Path) -> Result<usize, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    let jlo_path = root.join(JLO_DIR);
    let mut count = 0;

    // Collect all files under .jlo/ recursively and project to .jules/
    let files = collect_files_recursive(&jlo_path, &jlo_path)?;
    for (rel_path, content) in files {
        // Skip the version file and internal-only files
        if rel_path == VERSION_FILE {
            continue;
        }

        let jules_rel = format!("{}/{}", JULES_DIR, rel_path);
        ctx.workspace().write_file(&jules_rel, &content)?;
        count += 1;
    }

    Ok(count)
}

/// Delete workstreams in `.jules/workstreams/` that are absent from `.jlo/workstreams/`.
fn delete_absent_workstreams(root: &Path) -> Result<(), AppError> {
    let jlo_ws_dir = root.join(JLO_DIR).join("workstreams");
    let jules_ws_dir = root.join(JULES_DIR).join("workstreams");

    let jlo_workstreams = list_subdirs(&jlo_ws_dir);
    let jules_workstreams = list_subdirs(&jules_ws_dir);

    for ws in &jules_workstreams {
        if !jlo_workstreams.contains(ws) {
            let ws_path = jules_ws_dir.join(ws);
            std::fs::remove_dir_all(&ws_path).map_err(|e| {
                AppError::InternalError(format!(
                    "Failed to remove projected workstream {}: {}",
                    ws_path.display(),
                    e
                ))
            })?;
        }
    }

    Ok(())
}

/// Delete roles in `.jules/roles/<layer>/roles/` that are absent from `.jlo/roles/<layer>/roles/`.
fn delete_absent_roles(root: &Path) -> Result<(), AppError> {
    for layer in Layer::ALL {
        if layer.is_single_role() {
            continue;
        }

        let jlo_roles_dir = root.join(JLO_DIR).join("roles").join(layer.dir_name()).join("roles");
        let jules_roles_dir =
            root.join(JULES_DIR).join("roles").join(layer.dir_name()).join("roles");

        let jlo_roles = list_subdirs(&jlo_roles_dir);
        let jules_roles = list_subdirs(&jules_roles_dir);

        for role in &jules_roles {
            if !jlo_roles.contains(role) {
                let role_path = jules_roles_dir.join(role);
                std::fs::remove_dir_all(&role_path).map_err(|e| {
                    AppError::InternalError(format!(
                        "Failed to remove projected role {}: {}",
                        role_path.display(),
                        e
                    ))
                })?;
            }
        }
    }

    Ok(())
}

/// Recursively collect all files under a directory as (relative_path, content) pairs.
fn collect_files_recursive(base: &Path, dir: &Path) -> Result<Vec<(String, String)>, AppError> {
    let mut result = Vec::new();
    if !dir.exists() {
        return Ok(result);
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            result.extend(collect_files_recursive(base, &path)?);
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| AppError::InternalError(format!("Path strip failed: {}", e)))?;
            let content = std::fs::read_to_string(&path)?;
            result.push((rel.to_string_lossy().to_string(), content));
        }
    }

    Ok(result)
}

/// List subdirectory names (not paths) for a directory.
fn list_subdirs(dir: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                names.insert(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    names
}
