//! Matrix-producing workflow commands.
//!
//! These commands export GitHub Actions matrices for workflow orchestration.

mod pending;
mod routing;

pub use pending::{MatrixPendingOptions, MatrixPendingOutput};
pub use routing::{MatrixRoutingOptions, MatrixRoutingOutput};

use crate::domain::AppError;

/// Check flat exchange for pending events.
pub fn pending(options: MatrixPendingOptions) -> Result<MatrixPendingOutput, AppError> {
    let store = crate::adapters::workspace_filesystem::FilesystemWorkspaceStore::current()?;
    pending::execute(&store, options)
}

/// Export planner/implementer issue matrices from flat exchange and routing labels.
pub fn routing(options: MatrixRoutingOptions) -> Result<MatrixRoutingOutput, AppError> {
    let store = crate::adapters::workspace_filesystem::FilesystemWorkspaceStore::current()?;
    routing::execute(&store, options)
}
