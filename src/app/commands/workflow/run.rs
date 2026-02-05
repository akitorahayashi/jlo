//! Workflow run command implementation.
//!
//! Runs a layer and returns wait-gating metadata. Waiting is time-based, not PR-count based.

use chrono::Utc;
use serde::Serialize;

use crate::domain::{AppError, Layer};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

/// Options for workflow run command.
#[derive(Debug, Clone)]
pub struct WorkflowRunOptions {
    /// Target layer.
    pub layer: Layer,
    /// Matrix JSON input (required for non-narrator layers).
    pub matrix_json: Option<serde_json::Value>,
    /// Target branch for implementers.
    #[allow(dead_code)]
    pub target_branch: Option<String>,
    /// Run in mock mode.
    pub mock: bool,
}

/// Output of workflow run command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRunOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Timestamp when run started (RFC3339 UTC).
    pub run_started_at: String,
    /// Mock tag (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_tag: Option<String>,
    /// Mock PR numbers (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_pr_numbers: Option<Vec<u64>>,
    /// Mock branches (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_branches: Option<Vec<String>>,
}

/// Execute workflow run command.
pub fn execute(options: WorkflowRunOptions) -> Result<WorkflowRunOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Validate matrix is provided for layers that need it
    validate_matrix_requirement(&options)?;

    let run_started_at = Utc::now().to_rfc3339();

    if options.mock {
        // Mock mode requires JULES_MOCK_TAG
        let mock_tag = std::env::var("JULES_MOCK_TAG").map_err(|_| {
            AppError::Validation(
                "Mock mode requires JULES_MOCK_TAG environment variable".to_string(),
            )
        })?;

        if !mock_tag.contains("mock") {
            return Err(AppError::Validation(
                "JULES_MOCK_TAG must contain 'mock' substring".to_string(),
            ));
        }

        // In mock mode, we don't actually run anything - that's done by `jlo run --mock`
        // This command just returns the metadata needed for wait commands
        return Ok(WorkflowRunOutput {
            schema_version: 1,
            run_started_at,
            mock_tag: Some(mock_tag),
            mock_pr_numbers: Some(vec![]),
            mock_branches: Some(vec![]),
        });
    }

    // Non-mock mode: real execution would happen via jlo run
    // This command returns metadata for orchestration
    Ok(WorkflowRunOutput {
        schema_version: 1,
        run_started_at,
        mock_tag: None,
        mock_pr_numbers: None,
        mock_branches: None,
    })
}

/// Validate matrix is provided for layers that need it.
fn validate_matrix_requirement(options: &WorkflowRunOptions) -> Result<(), AppError> {
    match options.layer {
        Layer::Narrators => Ok(()),
        Layer::Observers | Layer::Deciders | Layer::Planners | Layer::Implementers => {
            if options.matrix_json.is_none() {
                return Err(AppError::MissingArgument(format!(
                    "Matrix JSON is required for layer '{}'",
                    options.layer.dir_name()
                )));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrator_does_not_require_matrix() {
        let options = WorkflowRunOptions {
            layer: Layer::Narrators,
            matrix_json: None,
            target_branch: None,
            mock: false,
        };

        assert!(validate_matrix_requirement(&options).is_ok());
    }

    #[test]
    fn observer_requires_matrix() {
        let options = WorkflowRunOptions {
            layer: Layer::Observers,
            matrix_json: None,
            target_branch: None,
            mock: false,
        };

        assert!(validate_matrix_requirement(&options).is_err());
    }

    #[test]
    fn observer_with_matrix_is_valid() {
        let matrix = serde_json::json!({
            "include": [{"workstream": "alpha", "role": "taxonomy"}]
        });

        let options = WorkflowRunOptions {
            layer: Layer::Observers,
            matrix_json: Some(matrix),
            target_branch: None,
            mock: false,
        };

        assert!(validate_matrix_requirement(&options).is_ok());
    }
}
