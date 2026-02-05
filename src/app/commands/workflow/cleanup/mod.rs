//! Workflow cleanup commands implementation.
//!
//! Provides cleanup operations for mock artifacts and processed issues.

pub mod mock;
pub mod processed_issue;

pub use mock::{WorkflowCleanupMockOptions, WorkflowCleanupMockOutput};
pub use processed_issue::{
    WorkflowCleanupProcessedIssueOptions, WorkflowCleanupProcessedIssueOutput,
};

use crate::domain::AppError;

/// Execute cleanup mock command.
pub fn cleanup_mock(
    options: WorkflowCleanupMockOptions,
) -> Result<WorkflowCleanupMockOutput, AppError> {
    mock::execute(options)
}

/// Execute cleanup processed-issue command.
pub fn cleanup_processed_issue(
    options: WorkflowCleanupProcessedIssueOptions,
) -> Result<WorkflowCleanupProcessedIssueOutput, AppError> {
    processed_issue::execute(options)
}
