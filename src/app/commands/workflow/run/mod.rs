//! Workflow run command implementation.
//!
//! Executes a layer by reading schedule settings from `.jlo/config.toml`
//! and running enabled roles.
//! This command provides orchestration for GitHub Actions workflows.

mod input;
pub mod issue_routing;
pub mod layer;
pub mod options;

use chrono::Utc;

use crate::domain::AppError;
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};

use self::layer::execute_layer;
pub use self::options::{WorkflowRunOptions, WorkflowRunOutput};

/// Execute workflow run command.
pub fn execute<G, H>(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore + Clone + Send + Sync + 'static),
    options: WorkflowRunOptions,
    git: &G,
    github: &H,
) -> Result<WorkflowRunOutput, AppError>
where
    G: Git,
    H: GitHub,
{
    if !store.jules_exists() {
        return Err(AppError::JulesNotFound);
    }

    let run_started_at = Utc::now().to_rfc3339();

    // Mock mode configuration
    let mock_tag = if options.mock {
        let tag = options.mock_tag.clone().ok_or_else(|| {
            AppError::Validation("Mock mode requires mock_tag in options".to_string())
        })?;

        if !tag.contains("mock") {
            return Err(AppError::Validation("mock_tag must contain 'mock' substring".to_string()));
        }
        Some(tag)
    } else {
        None
    };

    // Execute layer runs for all active roles
    let run_results = execute_layer(store, &options, git, github)?;

    Ok(WorkflowRunOutput {
        schema_version: 1,
        run_started_at,
        mock_tag,
        mock_pr_numbers: run_results.mock_pr_numbers,
        mock_branches: run_results.mock_branches,
    })
}
