//! Exchange area observation and cleanup commands.
//!
//! Provides inspect, publish-proposals, and clean sub-commands under
//! `jlo workflow exchange`.

pub mod clean;
pub mod inspect;
mod model;
pub mod publish_proposals;

pub use clean::{
    ExchangeCleanMockOptions, ExchangeCleanMockOutput, ExchangeCleanRequirementOptions,
    ExchangeCleanRequirementOutput,
};
pub use inspect::ExchangeInspectOptions;
pub use model::ExchangeInspectOutput;
pub use publish_proposals::{ExchangePublishProposalsOptions, ExchangePublishProposalsOutput};

use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{GitPort, JloStorePort, JulesStorePort, RepositoryFilesystemPort};

/// Execute exchange inspect command.
pub fn inspect(options: ExchangeInspectOptions) -> Result<ExchangeInspectOutput, AppError> {
    inspect::execute(options)
}

/// Execute exchange publish-proposals command.
pub fn publish_proposals(
    options: ExchangePublishProposalsOptions,
) -> Result<ExchangePublishProposalsOutput, AppError> {
    publish_proposals::execute(options)
}

/// Execute exchange clean requirement command.
pub fn clean_requirement(
    options: ExchangeCleanRequirementOptions,
) -> Result<ExchangeCleanRequirementOutput, AppError> {
    clean::clean_requirement(options)
}

/// Execute exchange clean requirement command with injected adapters.
pub fn clean_requirement_with_adapters<
    G: GitPort,
    W: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader,
>(
    options: ExchangeCleanRequirementOptions,
    workspace: &W,
    git: &G,
) -> Result<ExchangeCleanRequirementOutput, AppError> {
    clean::clean_requirement_with_adapters(options, workspace, git)
}

/// Execute exchange clean mock command.
pub fn clean_mock(options: ExchangeCleanMockOptions) -> Result<ExchangeCleanMockOutput, AppError> {
    clean::clean_mock(options)
}
