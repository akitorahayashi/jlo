//! Workspace clean operations: requirement cleanup and mock artifact removal.

pub mod mock;
pub mod requirement;

pub use mock::{WorkspaceCleanMockOptions, WorkspaceCleanMockOutput};
pub use requirement::{WorkspaceCleanRequirementOptions, WorkspaceCleanRequirementOutput};

use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{GitPort, JloStorePort, JulesStorePort, RepositoryFilesystemPort};

/// Execute workspace clean requirement command.
pub fn clean_requirement(
    options: WorkspaceCleanRequirementOptions,
) -> Result<WorkspaceCleanRequirementOutput, AppError> {
    requirement::execute(options)
}

pub fn clean_requirement_with_adapters<
    G: GitPort,
    W: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader,
>(
    options: WorkspaceCleanRequirementOptions,
    workspace: &W,
    git: &G,
) -> Result<WorkspaceCleanRequirementOutput, AppError> {
    requirement::execute_with_adapters(options, workspace, git)
}

/// Execute workspace clean mock command.
pub fn clean_mock(
    options: WorkspaceCleanMockOptions,
) -> Result<WorkspaceCleanMockOutput, AppError> {
    mock::execute(options)
}
