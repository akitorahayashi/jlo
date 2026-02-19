//! Workflow bootstrap commands.
//!
//! `workflow bootstrap` is split into explicit subcommands so workflow files
//! can show the execution order directly.

use std::path::Path;

use crate::domain::{AppError, JLO_DIR, VERSION_FILE};

pub mod managed_files;
pub mod worker_branch;

pub use managed_files::{
    WorkflowBootstrapManagedFilesOptions, WorkflowBootstrapManagedFilesOutput,
};
pub use worker_branch::{
    WorkflowBootstrapWorkerBranchOptions, WorkflowBootstrapWorkerBranchOutput,
};

pub(super) fn validate_control_plane_preconditions(root: &Path) -> Result<(), AppError> {
    let jlo_path = root.join(JLO_DIR);
    if !jlo_path.exists() {
        return Err(AppError::Validation(
            "Bootstrap requires .jlo/ control plane. Run 'jlo init' on your control branch first."
                .to_string(),
        ));
    }

    let jlo_version_path = jlo_path.join(VERSION_FILE);
    if !jlo_version_path.exists() {
        return Err(AppError::RepositoryIntegrity(
            "Missing .jlo/.jlo-version. Control plane is incomplete.".to_string(),
        ));
    }

    Ok(())
}
