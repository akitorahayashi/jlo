use std::path::PathBuf;

use crate::domain::Layer;

/// Target selection for run execution.
///
/// This model is domain-facing and excludes runtime/CLI execution flags.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific role to run (required for observers/innovators).
    pub role: Option<String>,
    /// Local requirement file path (required for requirement-driven layers: planner, implementer).
    pub requirement: Option<PathBuf>,
    /// Task file selector for innovators (expected: create_three_proposals).
    pub task: Option<String>,
}
