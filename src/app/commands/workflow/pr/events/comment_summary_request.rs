//! Workflow `pr comment-summary-request` command implementation.
//!
//! Posts or updates a managed bot comment on `jules-*` PRs requesting
//! the three-section summary template.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Options for `workflow pr comment-summary-request`.
#[derive(Debug, Clone)]
pub struct CommentSummaryRequestOptions {
    /// PR number to comment on.
    pub pr_number: u64,
}

/// Output of `workflow pr comment-summary-request`.
#[derive(Debug, Clone, Serialize)]
pub struct CommentSummaryRequestOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    pub target: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_id: Option<u64>,
}

/// Execute `pr comment-summary-request`.
pub fn execute(
    _github: &impl GitHubPort,
    options: CommentSummaryRequestOptions,
) -> Result<CommentSummaryRequestOutput, AppError> {
    // Full implementation in Stage 3.
    Ok(CommentSummaryRequestOutput {
        schema_version: 1,
        applied: false,
        skipped_reason: Some("not yet implemented".to_string()),
        target: options.pr_number,
        comment_id: None,
    })
}
