//! Run command implementation for executing Jules agents.

use std::path::Path;

use crate::adapters::jules_client_http::HttpJulesClient;
use crate::app::commands::workflow::workspace::{
    WorkspaceCleanRequirementOptions, clean_requirement_with_adapters,
};
pub use crate::domain::configuration::parse_config_content;
use crate::domain::configuration::{load_config, validate_mock_prerequisites};
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::layers::get_layer_strategy;
use crate::domain::layers::strategy::JulesClientFactory;
use crate::domain::{AppError, JulesApiConfig};
pub use crate::domain::{RunOptions, RunResult};
use crate::ports::{GitHubPort, GitPort, JulesClient, WorkspaceStore};

struct LazyClientFactory {
    config: JulesApiConfig,
}

impl JulesClientFactory for LazyClientFactory {
    fn create(&self) -> Result<Box<dyn JulesClient>, AppError> {
        let client = HttpJulesClient::from_env_with_config(&self.config)?;
        Ok(Box::new(client))
    }
}

/// Execute the run command.
pub fn execute<G, H, W>(
    jules_path: &Path,
    options: RunOptions,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    // Validate phase if provided (prevents path traversal)
    if let Some(ref phase) = options.phase
        && !validate_safe_path_component(phase)
    {
        return Err(AppError::Validation(format!(
            "Invalid phase '{}': must be a safe path component (e.g. 'creation', 'refinement')",
            phase,
        )));
    }

    // Load configuration
    let config = load_config(jules_path, workspace)?;

    if options.mock {
        validate_mock_prerequisites(&options)?;
    }

    // Create client factory
    let client_factory = LazyClientFactory { config: config.jules.clone() };

    // Get layer strategy
    let strategy = get_layer_strategy(options.layer);

    // Execute strategy
    let result =
        strategy.execute(jules_path, &options, &config, git, github, workspace, &client_factory)?;

    // Handle post-execution cleanup (e.g. Implementer requirement)
    if let Some(path) = result.cleanup_requirement.as_ref() {
        let path_str = path.to_string_lossy().to_string();
        match clean_requirement_with_adapters(
            WorkspaceCleanRequirementOptions { requirement_file: path_str },
            workspace,
            git,
        ) {
            Ok(cleanup_res) => {
                println!(
                    "✅ Cleaned requirement and source events ({} file(s) removed)",
                    cleanup_res.deleted_paths.len()
                );
            }
            Err(e) => {
                // Log warning but don't fail the run result, as the main task succeeded
                println!("⚠️ Failed to clean up requirement: {}", e);
            }
        }
    }

    Ok(result)
}
