//! Matrix-producing workflow commands.
//!
//! These commands export GitHub Actions matrices for workflow orchestration.

mod pending_workstreams;
mod roles;
mod routing;
mod workstreams;

pub use pending_workstreams::{MatrixPendingWorkstreamsOptions, MatrixPendingWorkstreamsOutput};
pub use roles::{MatrixRolesOptions, MatrixRolesOutput};
pub use routing::{MatrixRoutingOptions, MatrixRoutingOutput};
pub use workstreams::{MatrixWorkstreamsOptions, MatrixWorkstreamsOutput};

use crate::domain::AppError;

/// Export enabled workstreams as a GitHub Actions matrix.
pub fn workstreams(options: MatrixWorkstreamsOptions) -> Result<MatrixWorkstreamsOutput, AppError> {
    workstreams::execute(options)
}

/// Export enabled roles for a multi-role layer as a GitHub Actions matrix.
pub fn roles(options: MatrixRolesOptions) -> Result<MatrixRolesOutput, AppError> {
    roles::execute(options)
}

/// Export workstreams with pending events as a GitHub Actions matrix.
pub fn pending_workstreams(
    options: MatrixPendingWorkstreamsOptions,
) -> Result<MatrixPendingWorkstreamsOutput, AppError> {
    pending_workstreams::execute(options)
}

/// Export planner/implementer issue matrices from workstream inspection and routing labels.
pub fn routing(options: MatrixRoutingOptions) -> Result<MatrixRoutingOutput, AppError> {
    routing::execute(options)
}
