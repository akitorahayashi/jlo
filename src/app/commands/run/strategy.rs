use std::path::Path;

use super::RunRuntimeOptions;
use crate::domain::{AppError, ControlPlaneConfig, Layer, PromptAssetLoader, RunOptions};
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};

pub use crate::domain::{JulesClientFactory, RunResult};

/// A strategy for executing a specific layer.
pub trait LayerStrategy<W>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    /// Execute the layer.
    #[allow(clippy::too_many_arguments)]
    fn execute(
        &self,
        jules_path: &Path,
        target: &RunOptions,
        runtime: &RunRuntimeOptions,
        config: &ControlPlaneConfig,
        git: &dyn Git,
        github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError>;
}

/// Get the strategy for a specific layer.
pub fn get_layer_strategy<W>(layer: Layer) -> Box<dyn LayerStrategy<W>>
where
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
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
