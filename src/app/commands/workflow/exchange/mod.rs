//! Exchange inspection and cleanup commands.

pub mod clean_issue;
pub mod inspect;
mod model;
pub mod publish_proposals;

pub use clean_issue::{WorkflowExchangeCleanIssueOptions, WorkflowExchangeCleanIssueOutput};
pub use inspect::WorkflowExchangeInspectOptions;
pub use model::WorkflowExchangeInspectOutput;
pub use publish_proposals::{
    WorkflowExchangePublishProposalsOptions, WorkflowExchangePublishProposalsOutput,
};

use crate::domain::AppError;

pub fn inspect(
    options: WorkflowExchangeInspectOptions,
) -> Result<WorkflowExchangeInspectOutput, AppError> {
    inspect::execute(options)
}

pub fn clean_issue(
    options: WorkflowExchangeCleanIssueOptions,
) -> Result<WorkflowExchangeCleanIssueOutput, AppError> {
    clean_issue::execute(options)
}

pub fn publish_proposals(
    options: WorkflowExchangePublishProposalsOptions,
) -> Result<WorkflowExchangePublishProposalsOutput, AppError> {
    publish_proposals::execute(options)
}
