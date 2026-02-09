//! Workflow `issue label-innovator` command implementation.
//!
//! Applies `innovator` and `innovator/<persona>` labels to proposal issues.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Options for `workflow issue label-innovator`.
#[derive(Debug, Clone)]
pub struct LabelInnovatorOptions {
    /// Issue number to label.
    pub issue_number: u64,
    /// Persona name (e.g., "scout", "architect").
    pub persona: String,
}

/// Output of `workflow issue label-innovator`.
#[derive(Debug, Clone, Serialize)]
pub struct LabelInnovatorOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    pub target: u64,
    pub labels: Vec<String>,
}

/// Execute `issue label-innovator`.
pub fn execute(
    _github: &impl GitHubPort,
    options: LabelInnovatorOptions,
) -> Result<LabelInnovatorOutput, AppError> {
    // Full implementation in Stage 4.
    Ok(LabelInnovatorOutput {
        schema_version: 1,
        applied: false,
        skipped_reason: Some("not yet implemented".to_string()),
        target: options.issue_number,
        labels: vec!["innovator".to_string(), format!("innovator/{}", options.persona)],
    })
}
