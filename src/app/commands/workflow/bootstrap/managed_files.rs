//! Workflow bootstrap managed-files subcommand.
//!
//! Materializes managed runtime files from embedded scaffold assets.

use serde::Serialize;

use crate::adapters::catalogs::EmbeddedRoleTemplateStore;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::AppError;
use crate::ports::{JulesStore, RoleTemplateStore};

/// Options for `workflow bootstrap managed-files`.
#[derive(Debug)]
pub struct WorkflowBootstrapManagedFilesOptions {
    /// Root path of the repository.
    pub root: std::path::PathBuf,
}

/// Output of `workflow bootstrap managed-files`.
#[derive(Debug, Serialize)]
pub struct WorkflowBootstrapManagedFilesOutput {
    /// Whether scaffold projection was applied.
    pub applied: bool,
    /// jlo version stamped to `.jules/.jlo-version`.
    pub version: String,
    /// Number of write operations performed.
    pub files_written: usize,
}

/// Execute `workflow bootstrap managed-files`.
pub fn execute(
    options: WorkflowBootstrapManagedFilesOptions,
) -> Result<WorkflowBootstrapManagedFilesOutput, AppError> {
    super::validate_control_plane_preconditions(options.root.as_path())?;

    let repository = LocalRepositoryAdapter::new(options.root);
    let templates = EmbeddedRoleTemplateStore::new();
    let scaffold_files = templates.scaffold_files();
    repository.create_structure(&scaffold_files)?;

    let version = env!("CARGO_PKG_VERSION").to_string();
    repository.jules_write_version(&version)?;

    Ok(WorkflowBootstrapManagedFilesOutput {
        applied: true,
        version,
        files_written: scaffold_files.len() + 1,
    })
}
