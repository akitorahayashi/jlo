//! Workflow `pr process` pipeline command implementation.
//!
//! Runs event-level PR commands in configured order and emits per-step results.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Options for `workflow pr process`.
#[derive(Debug, Clone)]
pub struct ProcessOptions {
    /// PR number to process.
    pub pr_number: u64,
}

/// Per-step result inside the pipeline output.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessStepResult {
    pub command: String,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

/// Output of `workflow pr process`.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessOutput {
    pub schema_version: u32,
    pub target: u64,
    pub steps: Vec<ProcessStepResult>,
}

/// Execute `pr process`.
pub fn execute(
    _github: &impl GitHubPort,
    options: ProcessOptions,
) -> Result<ProcessOutput, AppError> {
    // Full implementation in Stage 5.
    Ok(ProcessOutput { schema_version: 1, target: options.pr_number, steps: Vec::new() })
}
