//! Run command implementation for executing Jules agents.

mod config;
mod multi_role;
mod prompt;
mod single_role;

use std::path::Path;
use std::path::PathBuf;

use crate::domain::{AppError, Layer};

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific roles to run (None = all from config). Only for multi-role layers.
    pub roles: Option<Vec<String>>,
    /// Show assembled prompts without executing.
    pub dry_run: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Local issue file path (required for single-role layers: planners, implementers).
    pub issue: Option<PathBuf>,
}

/// Result of a run execution.
#[derive(Debug)]
pub struct RunResult {
    /// Roles that were processed.
    pub roles: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Session IDs from Jules (empty if dry_run).
    pub sessions: Vec<String>,
}

/// Execute the run command.
pub fn execute(jules_path: &Path, options: RunOptions) -> Result<RunResult, AppError> {
    // Check if we are in CI environment
    let is_ci = std::env::var("GITHUB_ACTIONS").is_ok();

    // Single-role layers (Planners, Implementers) are issue-driven
    if options.layer.is_single_role() {
        let issue_path = options.issue.as_deref().ok_or_else(|| {
            AppError::ConfigError(
                "Issue path is required for single-role layers but was not provided.".to_string(),
            )
        })?;
        return single_role::execute(
            jules_path,
            options.layer,
            issue_path,
            options.dry_run,
            options.branch.as_deref(),
            is_ci,
        );
    }

    // Multi-role layers (Observers, Deciders)
    multi_role::execute(
        jules_path,
        options.layer,
        options.roles.as_ref(),
        options.dry_run,
        options.branch.as_deref(),
    )
}
