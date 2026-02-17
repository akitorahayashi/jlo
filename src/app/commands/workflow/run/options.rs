use crate::domain::Layer;
use serde::Serialize;

/// Options for workflow run command.
#[derive(Debug, Clone)]
pub struct WorkflowRunOptions {
    /// Target layer.
    pub layer: Layer,
    /// Run in mock mode.
    pub mock: bool,
    /// Optional starting branch override passed to `run`.
    pub branch: Option<String>,
    /// Mock tag (required if mock is true).
    pub mock_tag: Option<String>,
    /// Task selector for innovators (expected: create_three_proposals).
    pub task: Option<String>,
}

/// Output of workflow run command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRunOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Target layer that was executed.
    pub layer: Layer,
    /// Timestamp when run started (RFC3339 UTC).
    pub run_started_at: String,
    /// Number of API requests that succeeded during execution.
    pub number_of_api_requests_succeeded: u32,
    /// Reason the layer was skipped (present when success count is zero).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
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

/// Results from running a layer.
pub(crate) struct RunResults {
    /// Number of API requests that succeeded.
    pub(crate) number_of_api_requests_succeeded: u32,
    /// Reason the layer was skipped (present when nothing was executed).
    pub(crate) skip_reason: Option<String>,
    pub(crate) mock_pr_numbers: Option<Vec<u64>>,
    pub(crate) mock_branches: Option<Vec<String>>,
}

impl RunResults {
    /// Construct a skipped result with zero successes.
    pub(crate) fn skipped(reason: impl Into<String>) -> Self {
        Self {
            number_of_api_requests_succeeded: 0,
            skip_reason: Some(reason.into()),
            mock_pr_numbers: None,
            mock_branches: None,
        }
    }

    /// Construct a result with a success count and no skip reason.
    pub(crate) fn with_count(count: u32) -> Self {
        Self {
            number_of_api_requests_succeeded: count,
            skip_reason: if count == 0 { Some("No targets executed".to_string()) } else { None },
            mock_pr_numbers: None,
            mock_branches: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_run_options() {
        let options = WorkflowRunOptions {
            layer: Layer::Observers,
            mock: false,
            branch: None,
            mock_tag: None,
            task: None,
        };
        assert_eq!(options.layer, Layer::Observers);
        assert!(!options.mock);
    }

    #[test]
    fn workflow_run_output_serialization_contract() {
        let output = WorkflowRunOutput {
            schema_version: 1,
            layer: Layer::Decider,
            run_started_at: "2025-01-01T00:00:00Z".to_string(),
            number_of_api_requests_succeeded: 3,
            skip_reason: None,
            mock_tag: None,
            mock_pr_numbers: None,
            mock_branches: None,
        };
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["layer"], "decider");
        assert_eq!(parsed["number_of_api_requests_succeeded"], 3);
        assert!(parsed.get("skip_reason").is_none());
        assert!(parsed.get("mock_tag").is_none());
    }

    #[test]
    fn workflow_run_output_serialization_with_skip() {
        let output = WorkflowRunOutput {
            schema_version: 1,
            layer: Layer::Narrator,
            run_started_at: "2025-01-01T00:00:00Z".to_string(),
            number_of_api_requests_succeeded: 0,
            skip_reason: Some("No pending events".to_string()),
            mock_tag: None,
            mock_pr_numbers: None,
            mock_branches: None,
        };
        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["number_of_api_requests_succeeded"], 0);
        assert_eq!(parsed["skip_reason"], "No pending events");
    }

    #[test]
    fn run_results_skipped_sets_zero_count() {
        let r = RunResults::skipped("No enabled roles");
        assert_eq!(r.number_of_api_requests_succeeded, 0);
        assert_eq!(r.skip_reason.as_deref(), Some("No enabled roles"));
    }

    #[test]
    fn run_results_with_count_nonzero_has_no_skip_reason() {
        let r = RunResults::with_count(5);
        assert_eq!(r.number_of_api_requests_succeeded, 5);
        assert!(r.skip_reason.is_none());
    }

    #[test]
    fn run_results_with_count_zero_has_skip_reason() {
        let r = RunResults::with_count(0);
        assert_eq!(r.number_of_api_requests_succeeded, 0);
        assert!(r.skip_reason.is_some());
    }
}
