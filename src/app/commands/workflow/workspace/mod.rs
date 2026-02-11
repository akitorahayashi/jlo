//! Workspace observation and cleanup commands.
//!
//! Provides inspect, publish-proposals, and clean sub-commands under
//! `jlo workflow workspace`.

pub mod clean;
pub mod inspect;
mod model;
pub mod publish_proposals;

pub use clean::{
    WorkspaceCleanMockOptions, WorkspaceCleanMockOutput, WorkspaceCleanRequirementOptions,
    WorkspaceCleanRequirementOutput,
};
pub use inspect::WorkspaceInspectOptions;
pub use model::WorkspaceInspectOutput;
pub use publish_proposals::{WorkspacePublishProposalsOptions, WorkspacePublishProposalsOutput};

use crate::domain::AppError;

/// Execute workspace inspect command.
pub fn inspect(options: WorkspaceInspectOptions) -> Result<WorkspaceInspectOutput, AppError> {
    inspect::execute(options)
}

/// Execute workspace publish-proposals command.
pub fn publish_proposals(
    options: WorkspacePublishProposalsOptions,
) -> Result<WorkspacePublishProposalsOutput, AppError> {
    publish_proposals::execute(options)
}

/// Execute workspace clean requirement command.
pub fn clean_requirement(
    options: WorkspaceCleanRequirementOptions,
) -> Result<WorkspaceCleanRequirementOutput, AppError> {
    clean::clean_requirement(options)
}

/// Execute workspace clean mock command.
pub fn clean_mock(
    options: WorkspaceCleanMockOptions,
) -> Result<WorkspaceCleanMockOutput, AppError> {
    clean::clean_mock(options)
}
