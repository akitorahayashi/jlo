//! Workflow `pr sync-category-label` command implementation.
//!
//! Applies category label to implementer PRs by extracting the label
//! from branch naming and ensuring it exists with the configured color.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Options for `workflow pr sync-category-label`.
#[derive(Debug, Clone)]
pub struct SyncCategoryLabelOptions {
    /// PR number to label.
    pub pr_number: u64,
}

/// Output of `workflow pr sync-category-label`.
#[derive(Debug, Clone, Serialize)]
pub struct SyncCategoryLabelOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    pub target: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Execute `pr sync-category-label`.
pub fn execute(
    _github: &impl GitHubPort,
    options: SyncCategoryLabelOptions,
) -> Result<SyncCategoryLabelOutput, AppError> {
    // Full implementation in Stage 4.
    Ok(SyncCategoryLabelOutput {
        schema_version: 1,
        applied: false,
        skipped_reason: Some("not yet implemented".to_string()),
        target: options.pr_number,
        label: None,
    })
}
