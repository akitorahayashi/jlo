use std::path::{Path, PathBuf};

use crate::domain::{AppError, Layer, PromptAssetLoader, RunConfig, RunOptions};
use crate::ports::{
    GitHubPort, GitPort, JloStorePort, JulesClient, JulesStorePort, RepositoryFilesystemPort,
};

/// Result of a run execution.
#[derive(Debug)]
pub struct RunResult {
    /// Role that was processed.
    pub roles: Vec<String>,
    /// Whether this was a prompt preview.
    pub prompt_preview: bool,
    /// Session IDs from Jules (empty if prompt_preview or mock).
    pub sessions: Vec<String>,
    /// Requirement file to clean up (delete) after successful execution.
    pub cleanup_requirement: Option<PathBuf>,
}

/// Factory for creating a Jules client on demand.
pub trait JulesClientFactory {
    fn create(&self) -> Result<Box<dyn JulesClient>, AppError>;
}

/// A strategy for executing a specific layer.
pub trait LayerStrategy<W>
where
    W: RepositoryFilesystemPort + JloStorePort + JulesStorePort + PromptAssetLoader,
{
    /// Execute the layer.
    #[allow(clippy::too_many_arguments)]
    fn execute(
        &self,
        jules_path: &Path,
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn GitPort,
        github: &dyn GitHubPort,
        workspace: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError>;
}

/// Get the strategy for a specific layer.
pub fn get_layer_strategy<W>(layer: Layer) -> Box<dyn LayerStrategy<W>>
where
    W: RepositoryFilesystemPort
        + JloStorePort
        + JulesStorePort
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    match layer {
        Layer::Narrator => Box::new(super::layer::narrator::NarratorLayer),
        Layer::Decider => Box::new(super::layer::decider::DeciderLayer),
        Layer::Planner => Box::new(super::layer::planner::PlannerLayer),
        Layer::Implementer => Box::new(super::layer::implementer::ImplementerLayer),
        Layer::Observers => Box::new(super::layer::observers::ObserversLayer),
        Layer::Innovators => Box::new(super::layer::innovators::InnovatorsLayer),
        Layer::Integrator => Box::new(super::layer::integrator::IntegratorLayer),
    }
}
