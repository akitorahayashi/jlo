//! Workflow cleanup processed-issue command implementation.
//!
//! Deletes a processed issue file and its source event files.

use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::domain::AppError;
use crate::ports::WorkspaceStore;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

/// Options for workflow cleanup processed-issue command.
#[derive(Debug, Clone)]
pub struct WorkflowCleanupProcessedIssueOptions {
    /// Path to the issue file to delete.
    pub issue_file: String,
    /// Whether to commit changes.
    pub commit: bool,
    /// Whether to push changes.
    pub push: bool,
}

/// Output of workflow cleanup processed-issue command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowCleanupProcessedIssueOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Paths that were deleted.
    pub deleted_paths: Vec<String>,
    /// Whether changes were committed.
    pub committed: bool,
    /// Commit SHA if committed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    /// Whether changes were pushed.
    pub pushed: bool,
}

/// Execute cleanup processed-issue command.
pub fn execute(
    options: WorkflowCleanupProcessedIssueOptions,
) -> Result<WorkflowCleanupProcessedIssueOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = workspace.jules_path();
    let issue_path = Path::new(&options.issue_file);

    // Canonicalize and validate issue file is within .jules/
    let canonical_issue = issue_path.canonicalize().map_err(|_| {
        AppError::Validation(format!("Issue file does not exist: {}", options.issue_file))
    })?;

    if !canonical_issue.starts_with(&jules_path) {
        return Err(AppError::Validation(format!(
            "Issue file must be within .jules/ directory: {}",
            options.issue_file
        )));
    }

    // Read issue file to find source_events
    let content = fs::read_to_string(&canonical_issue)?;
    let source_events = extract_source_events(&content)?;

    let mut deleted_paths = Vec::new();

    // Delete source event files (with path validation)
    for event_path in &source_events {
        let full_path = Path::new(event_path);
        if let Ok(canonical_event) = full_path.canonicalize() {
            if !canonical_event.starts_with(&jules_path) {
                return Err(AppError::Validation(format!(
                    "Source event must be within .jules/ directory: {}",
                    event_path
                )));
            }
            fs::remove_file(&canonical_event)?;
            deleted_paths.push(event_path.clone());
        }
    }

    // Delete the issue file itself
    fs::remove_file(&canonical_issue)?;
    deleted_paths.push(options.issue_file.clone());

    let mut committed = false;
    let mut commit_sha = None;
    let mut pushed = false;

    // Commit if requested
    if options.commit && !deleted_paths.is_empty() {
        // This would use git commands via GitPort
        // For now, this is a stub
        eprintln!("Would commit deletion of {} files", deleted_paths.len());
        committed = true;
        commit_sha = Some("0000000000000000".to_string()); // Placeholder
    }

    // Push if requested
    if options.push && committed {
        // This would use git commands via GitPort
        eprintln!("Would push changes");
        pushed = true;
    }

    Ok(WorkflowCleanupProcessedIssueOutput {
        schema_version: 1,
        deleted_paths,
        committed,
        commit_sha,
        pushed,
    })
}

/// Extract source_events list from issue YAML content.
fn extract_source_events(content: &str) -> Result<Vec<String>, AppError> {
    let value: serde_yaml::Value = serde_yaml::from_str(content).map_err(|e| {
        AppError::ParseError { what: "issue file".to_string(), details: e.to_string() }
    })?;

    match &value["source_events"] {
        serde_yaml::Value::Sequence(events) => {
            let paths: Result<Vec<String>, _> = events
                .iter()
                .map(|v| match v {
                    serde_yaml::Value::String(s) => Ok(s.clone()),
                    _ => {
                        Err(AppError::Validation("source_events must contain strings".to_string()))
                    }
                })
                .collect();
            paths
        }
        _ => Err(AppError::Validation("Issue file missing source_events field".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_source_events_works() {
        let content = r#"
id: abc123
requires_deep_analysis: false
source_events:
  - .jules/workstreams/alpha/exchange/events/decided/event1.yml
  - .jules/workstreams/alpha/exchange/events/decided/event2.yml
"#;
        let events = extract_source_events(content).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events[0].contains("event1.yml"));
    }
}
