//! Workflow cleanup commands implementation.
//!
//! Provides cleanup operations for mock artifacts.

pub mod mock;

pub use mock::{WorkflowCleanupMockOptions, WorkflowCleanupMockOutput};

use crate::domain::AppError;

/// Execute cleanup mock command.
pub fn cleanup_mock(
    options: WorkflowCleanupMockOptions,
) -> Result<WorkflowCleanupMockOutput, AppError> {
    mock::execute(options)
}
