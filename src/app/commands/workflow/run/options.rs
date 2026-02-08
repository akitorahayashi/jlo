use crate::domain::Layer;
use serde::Serialize;

/// Options for workflow run command.
#[derive(Debug, Clone)]
pub struct WorkflowRunOptions {
    /// Target workstream.
    pub workstream: String,
    /// Target layer.
    pub layer: Layer,
    /// Run in mock mode.
    pub mock: bool,
    /// Mock tag (required if mock is true).
    pub mock_tag: Option<String>,
    /// Routing labels (optional override).
    pub routing_labels: Option<Vec<String>>,
    /// Execution phase for innovators (creation or refinement).
    pub phase: Option<String>,
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

/// Results from running a layer.
pub(crate) struct RunResults {
    pub(crate) mock_pr_numbers: Option<Vec<u64>>,
    pub(crate) mock_branches: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_run_options_with_workstream() {
        let options = WorkflowRunOptions {
            workstream: "generic".to_string(),
            layer: Layer::Observers,
            mock: false,
            mock_tag: None,
            routing_labels: None,
            phase: None,
        };
        assert_eq!(options.workstream, "generic");
        assert_eq!(options.layer, Layer::Observers);
        assert!(!options.mock);
    }
}
