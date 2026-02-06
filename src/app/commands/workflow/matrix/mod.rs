//! Matrix-producing workflow commands.
//!
//! These commands export GitHub Actions matrices for workflow orchestration.

mod pending_workstreams;
mod routing;
mod workstreams;

pub use pending_workstreams::{
    MatrixPendingWorkstreamsOptions, MatrixPendingWorkstreamsOutput,
    WorkstreamsMatrix as PendingWorkstreamsInput,
};
pub use routing::{
    MatrixRoutingOptions, MatrixRoutingOutput, WorkstreamsMatrix as RoutingWorkstreamsInput,
};
pub use workstreams::{MatrixWorkstreamsOptions, MatrixWorkstreamsOutput};

use crate::domain::AppError;

/// Export enabled workstreams as a GitHub Actions matrix.
pub fn workstreams(options: MatrixWorkstreamsOptions) -> Result<MatrixWorkstreamsOutput, AppError> {
    workstreams::execute(options)
}

/// Export workstreams with pending events as a GitHub Actions matrix.
pub fn pending_workstreams(
    options: MatrixPendingWorkstreamsOptions,
) -> Result<MatrixPendingWorkstreamsOutput, AppError> {
    pending_workstreams::execute(options)
}

/// Export planner/implementer issue matrices from workstream inspection and routing labels.
pub fn routing(options: MatrixRoutingOptions) -> Result<MatrixRoutingOutput, AppError> {
    let store =
        crate::adapters::workspace_filesystem::FilesystemWorkspaceStore::current()?;
    routing::execute(&store, options)
}
