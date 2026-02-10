//! Matrix-producing workflow commands.
//!
//! These commands export GitHub Actions matrices for workflow orchestration.

mod pending_workstreams;
mod routing;
mod workstreams;

pub use pending_workstreams::{MatrixPendingWorkstreamsOptions, MatrixPendingWorkstreamsOutput};
pub use routing::{MatrixRoutingOptions, MatrixRoutingOutput};
pub use workstreams::{MatrixWorkstreamsOptions, MatrixWorkstreamsOutput};

use crate::domain::AppError;

/// Export enabled workstreams as a GitHub Actions matrix.
pub fn workstreams(options: MatrixWorkstreamsOptions) -> Result<MatrixWorkstreamsOutput, AppError> {
    let store = crate::adapters::workspace_filesystem::FilesystemWorkspaceStore::current()?;
    workstreams::execute(&store, options)
}

/// Check flat exchange for pending events.
pub fn pending_workstreams(
    options: MatrixPendingWorkstreamsOptions,
) -> Result<MatrixPendingWorkstreamsOutput, AppError> {
    let store = crate::adapters::workspace_filesystem::FilesystemWorkspaceStore::current()?;
    pending_workstreams::execute(&store, options)
}

/// Export planner/implementer issue matrices from flat exchange and routing labels.
pub fn routing(options: MatrixRoutingOptions) -> Result<MatrixRoutingOutput, AppError> {
    let store = crate::adapters::workspace_filesystem::FilesystemWorkspaceStore::current()?;
    routing::execute(&store, options)
}
