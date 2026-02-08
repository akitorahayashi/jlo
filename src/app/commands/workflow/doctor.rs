//! Workflow doctor command implementation.
//!
//! Validates `.jules/` workspace structure for workflow automation.

use serde::Serialize;

use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Options for workflow doctor command.
#[derive(Debug, Clone, Default)]
pub struct WorkflowDoctorOptions {
    /// Limit checks to a specific workstream.
    pub workstream: Option<String>,
}

/// Output of workflow doctor command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowDoctorOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Whether all checks passed.
    pub ok: bool,
}

/// Execute workflow doctor validation.
///
/// Returns a machine-readable output indicating workspace health.
pub fn execute(options: WorkflowDoctorOptions) -> Result<WorkflowDoctorOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Delegate to existing doctor logic but translate to workflow output
    let doctor_options = crate::app::commands::doctor::DoctorOptions {
        strict: true, // Workflow mode is strict by default
        workstream: options.workstream,
    };

    let outcome = crate::app::commands::doctor::execute(&workspace.jules_path(), doctor_options)?;

    Ok(WorkflowDoctorOutput { schema_version: 1, ok: outcome.errors == 0 && outcome.warnings == 0 })
}
