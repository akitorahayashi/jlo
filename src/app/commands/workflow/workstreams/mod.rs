//! Workstream inspection and cleanup commands.

pub mod clean_issue;
pub mod inspect;
mod model;

pub use clean_issue::{WorkflowWorkstreamsCleanIssueOptions, WorkflowWorkstreamsCleanIssueOutput};
pub use inspect::WorkflowWorkstreamsInspectOptions;
pub use model::WorkflowWorkstreamsInspectOutput;

use crate::domain::AppError;

pub fn inspect(
    options: WorkflowWorkstreamsInspectOptions,
) -> Result<WorkflowWorkstreamsInspectOutput, AppError> {
    inspect::execute(options)
}

pub fn clean_issue(
    options: WorkflowWorkstreamsCleanIssueOptions,
) -> Result<WorkflowWorkstreamsCleanIssueOutput, AppError> {
    clean_issue::execute(options)
}
