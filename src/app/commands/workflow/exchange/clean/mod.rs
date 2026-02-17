//! Exchange clean operations: requirement cleanup and mock artifact removal.

pub mod mock;
pub mod requirement;

pub use mock::{ExchangeCleanMockOptions, ExchangeCleanMockOutput};
pub use requirement::{
    ExchangeCleanRequirementApplyOutput, ExchangeCleanRequirementOptions,
    ExchangeCleanRequirementOutput,
};

use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{Git, JloStore, JulesStore, RepositoryFilesystem};

/// Execute exchange clean requirement command.
pub fn clean_requirement(
    options: ExchangeCleanRequirementOptions,
) -> Result<ExchangeCleanRequirementOutput, AppError> {
    requirement::execute(options)
}

pub fn clean_requirement_apply_with_adapters<
    G: Git,
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
>(
    options: ExchangeCleanRequirementOptions,
    repository: &W,
    git: &G,
) -> Result<ExchangeCleanRequirementApplyOutput, AppError> {
    requirement::apply_with_adapters(options, repository, git)
}

/// Execute exchange clean mock command.
pub fn clean_mock(options: ExchangeCleanMockOptions) -> Result<ExchangeCleanMockOutput, AppError> {
    mock::execute(options)
}
