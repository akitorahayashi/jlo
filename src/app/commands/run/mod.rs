//! Run command implementation for executing Jules agents.

mod config;
mod config_dto;
mod decider;
mod implementer;
mod innovators;
mod issue_execution;
pub mod mock;
mod multi_role_execution;
pub mod narrator;
pub(crate) mod narrator_logic;
mod observers;
mod planner;
mod prompt;

pub use config::parse_config_content;

use std::path::Path;
use std::path::PathBuf;

use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::{AppError, Layer};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific role to run (required for observers/decider/innovators).
    pub role: Option<String>,
    /// Show assembled prompts without executing.
    pub prompt_preview: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Local issue file path (required for issue-driven layers: planner, implementer).
    pub issue: Option<PathBuf>,
    /// Run in mock mode (no Jules API, real git/GitHub operations).
    pub mock: bool,
    /// Execution phase for innovators (creation or refinement).
    pub phase: Option<String>,
}

/// Result of a run execution.
#[derive(Debug)]
pub struct RunResult {
    /// Role that was processed.
    pub roles: Vec<String>,
    /// Whether this was a prompt preview.
    pub prompt_preview: bool,
    /// Session IDs from Jules (empty if prompt_preview or mock).
    pub sessions: Vec<String>,
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
    // Handle mock mode
    if options.mock {
        return mock::execute(jules_path, &options, git, github, workspace);
    }

    // Validate phase if provided (prevents path traversal)
    if let Some(ref phase) = options.phase
        && !validate_safe_path_component(phase)
    {
        return Err(AppError::Validation(format!(
            "Invalid phase '{}': must be a safe path component (e.g. 'creation', 'refinement')",
            phase,
        )));
    }

    // Narrator is single-role but not issue-driven
    if options.layer == Layer::Narrator {
        return narrator::execute(
            jules_path,
            options.prompt_preview,
            options.branch.as_deref(),
            git,
            workspace,
        );
    }

    // Decider is single-role (no --role required, prompt resolves without role variable)
    if options.layer == Layer::Decider {
        return decider::execute(jules_path, &options, workspace);
    }

    // Check if we are in CI environment (for issue-driven and single-role layers)
    let is_ci = std::env::var("GITHUB_ACTIONS").is_ok();

    // Issue-driven layers (Planner, Implementer) require an issue path
    if options.layer.is_issue_driven() {
        let issue_path = options.issue.as_deref().ok_or_else(|| {
            AppError::MissingArgument(
                "Issue path is required for issue-driven layers but was not provided.".to_string(),
            )
        })?;
        return match options.layer {
            Layer::Planner => {
                planner::execute(jules_path, &options, issue_path, is_ci, github, workspace)
            }
            Layer::Implementer => {
                implementer::execute(jules_path, &options, issue_path, is_ci, github, workspace)
            }
            _ => Err(AppError::Validation(format!(
                "Unexpected issue-driven layer '{}'",
                options.layer.dir_name()
            ))),
        };
    }

    // Layer-specific multi-role execution
    match options.layer {
        Layer::Observers => observers::execute(jules_path, &options, workspace),
        Layer::Innovators => innovators::execute(jules_path, &options, workspace),
        _ => Err(AppError::Validation(format!(
            "Unexpected layer '{}' reached multi-role dispatch",
            options.layer.dir_name()
        ))),
    }
}
