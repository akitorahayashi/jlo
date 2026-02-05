//! Run command implementation for executing Jules agents.

mod config;
mod config_dto;
pub mod mock;
mod multi_role;

pub use config::parse_config_content;
pub mod narrator;
mod prompt;
mod role_selection;
pub mod single_role;

use std::path::Path;
use std::path::PathBuf;

use crate::domain::{AppError, Layer};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific roles to run (None = all from config). Only for multi-role layers.
    pub roles: Option<Vec<String>>,
    /// Workstream for multi-role layers.
    pub workstream: Option<String>,
    /// Use scheduled mode for multi-role layers.
    pub scheduled: bool,
    /// Show assembled prompts without executing.
    pub dry_run: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Local issue file path (required for issue-driven layers: planners, implementers).
    pub issue: Option<PathBuf>,
    /// Run in mock mode (no Jules API, real git/GitHub operations).
    pub mock: bool,
}

/// Result of a run execution.
#[derive(Debug)]
pub struct RunResult {
    /// Roles that were processed.
    pub roles: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Session IDs from Jules (empty if dry_run or mock).
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
    W: WorkspaceStore,
{
    // Handle mock mode
    if options.mock {
        return mock::execute(jules_path, &options, git, github, workspace);
    }

    // Check if we are in CI environment
    let is_ci = std::env::var("GITHUB_ACTIONS").is_ok();

    // Narrator is single-role but not issue-driven
    if options.layer == Layer::Narrators {
        return narrator::execute(
            options.dry_run,
            options.branch.as_deref(),
            is_ci,
            git,
            workspace,
        );
    }

    // Issue-driven layers (Planners, Implementers) require an issue path
    if options.layer.is_issue_driven() {
        let issue_path = options.issue.as_deref().ok_or_else(|| {
            AppError::MissingArgument(
                "Issue path is required for issue-driven layers but was not provided.".to_string(),
            )
        })?;
        return single_role::execute(
            jules_path,
            options.layer,
            issue_path,
            options.dry_run,
            options.branch.as_deref(),
            is_ci,
            github,
            workspace,
        );
    }

    // Multi-role layers (Observers, Deciders)
    multi_role::execute(
        jules_path,
        options.layer,
        options.roles.as_ref(),
        options.workstream.as_deref(),
        options.scheduled,
        options.dry_run,
        options.branch.as_deref(),
        workspace,
    )
}
