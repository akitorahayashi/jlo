//! Workflow `pr enable-automerge` command implementation.
//!
//! Evaluates auto-merge policy gates and enables auto-merge on eligible PRs.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Options for `workflow pr enable-automerge`.
#[derive(Debug, Clone)]
pub struct EnableAutomergeOptions {
    /// PR number to enable auto-merge on.
    pub pr_number: u64,
}

/// Output of `workflow pr enable-automerge`.
#[derive(Debug, Clone, Serialize)]
pub struct EnableAutomergeOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    pub target: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automerge_state: Option<String>,
}

/// Execute `pr enable-automerge`.
pub fn execute(
    _github: &impl GitHubPort,
    options: EnableAutomergeOptions,
) -> Result<EnableAutomergeOutput, AppError> {
    // Full implementation in Stage 5.
    Ok(EnableAutomergeOutput {
        schema_version: 1,
        applied: false,
        skipped_reason: Some("not yet implemented".to_string()),
        target: options.pr_number,
        automerge_state: None,
    })
}
