//! Workflow bootstrap: deterministic projection of `.jules/` from `.jlo/` + scaffold.
//!
//! Runs as the first step in workflow execution, guaranteeing that the runtime
//! workspace matches the control-plane intent before any agent job.
//!
//! Invariants:
//! - Missing `.jlo/` is a hard failure.
//! - Missing `.jlo/.jlo-version` is a hard failure.
//! - Managed framework files are always materialized from the embedded scaffold.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::adapters::embedded_role_template_store::EmbeddedRoleTemplateStore;
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::app::AppContext;
use crate::domain::workspace::manifest::{MANIFEST_FILENAME, hash_content, is_default_role_file};
use crate::domain::workspace::workspace_layout::{JLO_DIR, JULES_DIR, VERSION_FILE};
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

/// Full deterministic projection of `.jules/` from embedded scaffold.
fn project_runtime<W, R>(
    ctx: &AppContext<W, R>,
    _root: &Path,
    version: &str,
) -> Result<usize, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    // Counts write operations performed.
    let mut files_written: usize = 0;

    // 1. Materialize managed framework files from embedded scaffold
    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;
    ctx.workspace().write_version(version)?;
    files_written += scaffold_files.len() + 1; // +1 for version

    // 2. Write managed manifest
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
