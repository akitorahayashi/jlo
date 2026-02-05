//! Workflow run command implementation.
//!
//! Runs a layer sequentially using a matrix input and returns wait-gating metadata.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::domain::{AppError, Layer};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

/// Options for workflow run command.
#[derive(Debug, Clone)]
pub struct WorkflowRunOptions {
    /// Target layer.
    pub layer: Layer,
    /// Matrix JSON input (from matrix commands).
    pub matrix_json: Option<MatrixInput>,
    /// Target branch for implementers.
    #[allow(dead_code)]
    pub target_branch: Option<String>,
    /// Run in mock mode.
    pub mock: bool,
}

/// Input matrix structure.
#[derive(Debug, Clone, Deserialize)]
pub struct MatrixInput {
    /// Matrix include entries.
    pub include: Vec<serde_json::Value>,
}

/// Output of workflow run command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRunOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Expected number of PRs to wait for.
    pub expected_count: usize,
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

    // Calculate expected count based on layer and matrix
    let expected_count = calculate_expected_count(&options)?;

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
        // For now, return empty arrays that will be populated by actual mock execution
        return Ok(WorkflowRunOutput {
            schema_version: 1,
            expected_count,
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
        expected_count,
        run_started_at,
        mock_tag: None,
        mock_pr_numbers: None,
        mock_branches: None,
    })
}

/// Calculate expected PR count based on layer and matrix.
fn calculate_expected_count(options: &WorkflowRunOptions) -> Result<usize, AppError> {
    match options.layer {
        Layer::Narrators => {
            // Narrator is always 1
            Ok(1)
        }
        Layer::Observers | Layer::Planners | Layer::Implementers => {
            // These layers expect one PR per matrix entry
            match &options.matrix_json {
                Some(matrix) => Ok(matrix.include.len()),
                None => Err(AppError::MissingArgument(format!(
                    "Matrix JSON is required for layer '{}'",
                    options.layer.dir_name()
                ))),
            }
        }
        Layer::Deciders => {
            // Deciders: one PR per workstream (not per role)
            // Count unique workstreams in the matrix
            match &options.matrix_json {
                Some(matrix) => {
                    let unique_workstreams: std::collections::HashSet<String> = matrix
                        .include
                        .iter()
                        .filter_map(|entry| entry.get("workstream").and_then(|v| v.as_str()))
                        .map(|s| s.to_string())
                        .collect();
                    Ok(unique_workstreams.len())
                }
                None => Err(AppError::MissingArgument(
                    "Matrix JSON is required for deciders layer".to_string(),
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrator_expected_count_is_one() {
        let options = WorkflowRunOptions {
            layer: Layer::Narrators,
            matrix_json: None,
            target_branch: None,
            mock: false,
        };

        // Narrators always expect 1, test the calculation logic directly
        let count = calculate_expected_count(&options).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn observer_expected_count_from_matrix() {
        let matrix = MatrixInput {
            include: vec![
                serde_json::json!({"workstream": "alpha", "role": "taxonomy"}),
                serde_json::json!({"workstream": "alpha", "role": "qa"}),
            ],
        };

        let options = WorkflowRunOptions {
            layer: Layer::Observers,
            matrix_json: Some(matrix),
            target_branch: None,
            mock: false,
        };

        // This test would fail without a workspace, so we skip actual execution
        // and just test the calculation logic directly
        let count = calculate_expected_count(&options).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn decider_expected_count_by_workstream() {
        let matrix = MatrixInput {
            include: vec![
                serde_json::json!({"workstream": "alpha", "role": "triage"}),
                serde_json::json!({"workstream": "alpha", "role": "other"}),
                serde_json::json!({"workstream": "beta", "role": "triage"}),
            ],
        };

        let options = WorkflowRunOptions {
            layer: Layer::Deciders,
            matrix_json: Some(matrix),
            target_branch: None,
            mock: false,
        };

        let count = calculate_expected_count(&options).unwrap();
        // 2 unique workstreams (alpha, beta)
        assert_eq!(count, 2);
    }
}
