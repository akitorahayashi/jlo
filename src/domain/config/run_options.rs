use std::path::PathBuf;

use crate::domain::Layer;

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific role to run (required for observers/innovators).
    pub role: Option<String>,
    /// Show assembled prompts without executing.
    pub prompt_preview: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Local requirement file path (required for requirement-driven layers: planner, implementer).
    pub requirement: Option<PathBuf>,
    /// Run in mock mode (no Jules API, real git/GitHub operations).
    pub mock: bool,
    /// Task file selector for innovators (expected: create_three_proposals).
    pub task: Option<String>,
    /// Skip post-execution cleanup (requirement deletion and worker-branch push).
    pub no_cleanup: bool,
}
