//! Exchange clean operations: requirement cleanup and mock artifact removal.

pub mod mock;
pub mod requirement;

pub use mock::{ExchangeCleanMockOptions, ExchangeCleanMockOutput};
pub use requirement::{ExchangeCleanRequirementOptions, ExchangeCleanRequirementOutput};

use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{GitPort, JloStorePort, JulesStorePort, RepositoryFilesystemPort};

/// Execute exchange clean requirement command.
pub fn clean_requirement(
    options: ExchangeCleanRequirementOptions,
) -> Result<ExchangeCleanRequirementOutput, AppError> {
    requirement::execute(options)
}

pub fn clean_requirement_with_adapters<
    G: GitPort,
    W: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader,
>(
    options: ExchangeCleanRequirementOptions,
    workspace: &W,
    git: &G,
) -> Result<ExchangeCleanRequirementOutput, AppError> {
    requirement::execute_with_adapters(options, workspace, git)
}

/// Execute exchange clean mock command.
pub fn clean_mock(options: ExchangeCleanMockOptions) -> Result<ExchangeCleanMockOutput, AppError> {
    mock::execute(options)
}
