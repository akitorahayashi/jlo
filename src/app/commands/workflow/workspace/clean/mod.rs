//! Workspace clean operations: requirement cleanup and mock artifact removal.

pub mod mock;
pub mod requirement;

pub use mock::{WorkspaceCleanMockOptions, WorkspaceCleanMockOutput};
pub use requirement::{WorkspaceCleanRequirementOptions, WorkspaceCleanRequirementOutput};

use crate::domain::AppError;

/// Execute workspace clean requirement command.
pub fn clean_requirement(
    options: WorkspaceCleanRequirementOptions,
) -> Result<WorkspaceCleanRequirementOutput, AppError> {
    requirement::execute(options)
}

/// Execute workspace clean mock command.
pub fn clean_mock(
    options: WorkspaceCleanMockOptions,
) -> Result<WorkspaceCleanMockOutput, AppError> {
    mock::execute(options)
}
