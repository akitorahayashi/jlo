//! Workflow bootstrap: materialize `.jules/` runtime workspace on the `jules` branch.
//!
//! Runs as the first step in workflow execution, guaranteeing that the runtime
//! workspace exists and matches the current jlo version before any agent job.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::adapters::embedded_role_template_store::EmbeddedRoleTemplateStore;
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::app::AppContext;
use crate::domain::workspace::manifest::{MANIFEST_FILENAME, hash_content, is_default_role_file};
use crate::domain::{AppError, ScaffoldManifest};
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
/// Verifies that `.jules/` exists with the correct version. If missing or stale,
/// materializes the runtime workspace from the embedded scaffold.
pub fn execute(options: WorkflowBootstrapOptions) -> Result<WorkflowBootstrapOutput, AppError> {
    let current_version = env!("CARGO_PKG_VERSION");
    let root = &options.root;

    let store = FilesystemWorkspaceStore::new(root.clone());
    let templates = EmbeddedRoleTemplateStore::new();

    // Check if .jules/ exists with correct version
    if store.exists() {
        if let Some(existing_version) = read_runtime_version(root) {
            if existing_version == current_version {
                return Ok(WorkflowBootstrapOutput {
                    materialized: false,
                    version: current_version.to_string(),
                    files_written: 0,
                });
            }
            eprintln!(
                "Runtime version mismatch: {} (on branch) vs {} (jlo binary). Re-materializing.",
                existing_version, current_version
            );
        } else {
            eprintln!("Runtime version file missing. Re-materializing.");
        }
    } else {
        eprintln!(".jules/ not found on branch. Materializing runtime workspace.");
    }

    let ctx = AppContext::new(store, templates);
    let files_written = materialize_runtime(&ctx)?;

    Ok(WorkflowBootstrapOutput {
        materialized: true,
        version: current_version.to_string(),
        files_written,
    })
}

/// Read the `.jlo-version` from the `.jules/` runtime workspace.
fn read_runtime_version(root: &Path) -> Option<String> {
    let version_path = root.join(".jules/.jlo-version");
    std::fs::read_to_string(version_path).ok().map(|s| s.trim().to_string())
}

/// Materialize `.jules/` from embedded scaffold and write version + manifest.
fn materialize_runtime<W, R>(ctx: &AppContext<W, R>) -> Result<usize, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;
    ctx.workspace().write_version(env!("CARGO_PKG_VERSION"))?;

    // Create managed manifest for .jules/
    let mut map = BTreeMap::new();
    for file in &scaffold_files {
        if is_default_role_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    let managed_manifest = ScaffoldManifest::from_map(map);
    let manifest_content = managed_manifest.to_yaml()?;
    let manifest_path = format!(".jules/{}", MANIFEST_FILENAME);
    ctx.workspace().write_file(&manifest_path, &manifest_content)?;

    // +2 for version file and manifest
    Ok(scaffold_files.len() + 2)
}
