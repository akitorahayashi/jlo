//! Workflow wait prs command implementation.
//!
//! Waits for PR readiness conditions using time-based wait, and returns discovered PRs.

use serde::Serialize;

use crate::domain::{AppError, Layer};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

/// Options for workflow wait prs command.
#[derive(Debug, Clone)]
pub struct WorkflowWaitPrsOptions {
    /// Target layer (used to resolve branch_prefix from contracts).
    pub layer: Layer,
    /// Base branch for PR discovery.
    #[allow(dead_code)]
    pub base_branch: String,
    /// Run started timestamp (RFC3339 UTC).
    #[allow(dead_code)]
    pub run_started_at: String,
    /// Maximum wait time in minutes.
    pub wait_minutes: u32,
    /// Wait mode: merge or label.
    pub mode: WaitMode,
    /// Mock mode (overrides timeout to 30 seconds).
    pub mock: bool,
    /// Mock PR numbers to use instead of discovery.
    pub mock_pr_numbers_json: Option<Vec<u64>>,
}

/// Wait mode for PR readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitMode {
    /// Wait until PRs are merged.
    Merge,
    /// Wait until PRs have specific labels.
    Label,
}

impl WaitMode {
    /// Parse wait mode from string.
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "merge" => Ok(WaitMode::Merge),
            "label" => Ok(WaitMode::Label),
            _ => Err(AppError::Validation(format!(
                "Invalid wait mode '{}': must be 'merge' or 'label'",
                s
            ))),
        }
    }
}

/// Output of workflow wait prs command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowWaitPrsOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Discovered PR numbers.
    pub pr_numbers: Vec<u64>,
    /// Discovered PR head branches.
    pub pr_heads: Vec<String>,
}

/// Execute workflow wait prs command.
pub fn execute(options: WorkflowWaitPrsOptions) -> Result<WorkflowWaitPrsOutput, AppError> {
    // Require GH_TOKEN and GITHUB_REPOSITORY
    if std::env::var("GH_TOKEN").is_err() {
        return Err(AppError::Validation(
            "GH_TOKEN environment variable is required for wait prs".to_string(),
        ));
    }
    if std::env::var("GITHUB_REPOSITORY").is_err() {
        return Err(AppError::Validation(
            "GITHUB_REPOSITORY environment variable is required for wait prs".to_string(),
        ));
    }

    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // If mock PR numbers provided, use them directly
    if let Some(mock_prs) = options.mock_pr_numbers_json {
        return Ok(WorkflowWaitPrsOutput {
            schema_version: 1,
            pr_numbers: mock_prs.clone(),
            pr_heads: mock_prs.iter().map(|n| format!("mock-branch-{}", n)).collect(),
        });
    }

    // Load branch prefix from contracts.yml
    let _branch_prefix = load_branch_prefix(&workspace.jules_path(), options.layer)?;

    // Calculate actual timeout
    let timeout_seconds = if options.mock {
        30 // Mock mode always uses 30 seconds
    } else {
        options.wait_minutes as u64 * 60
    };

    // For now, this is a stub that would need actual GitHub API integration
    // The real implementation would:
    // 1. Query GitHub API for open PRs with matching branch prefix
    // 2. Filter by base_branch and creation time (after run_started_at)
    // 3. Wait until timeout (time-based wait is more reliable than PR counting)
    // 4. Apply mode-specific readiness check (merged or labeled)

    eprintln!("Waiting with timeout {}s (mode: {:?})", timeout_seconds, options.mode);

    // Placeholder: return empty result
    // Real implementation would use GitHub adapter
    Ok(WorkflowWaitPrsOutput { schema_version: 1, pr_numbers: vec![], pr_heads: vec![] })
}

/// Load branch prefix from contracts.yml for a layer.
fn load_branch_prefix(jules_path: &std::path::Path, layer: Layer) -> Result<String, AppError> {
    let contracts_path = jules_path.join("roles").join(layer.dir_name()).join("contracts.yml");

    let content = std::fs::read_to_string(&contracts_path).map_err(|_| {
        AppError::Validation(format!(
            "Missing contracts.yml for layer '{}': {}",
            layer.dir_name(),
            contracts_path.display()
        ))
    })?;

    extract_branch_prefix(&content).ok_or_else(|| {
        AppError::Validation(format!(
            "Missing branch_prefix in contracts.yml for layer '{}'",
            layer.dir_name()
        ))
    })
}

/// Extract branch_prefix from contracts.yml content.
fn extract_branch_prefix(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("branch_prefix:") {
            let value = line.trim_start_matches("branch_prefix:").trim();
            let value = value.trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_mode_parsing() {
        assert_eq!(WaitMode::from_str("merge").unwrap(), WaitMode::Merge);
        assert_eq!(WaitMode::from_str("label").unwrap(), WaitMode::Label);
        assert!(WaitMode::from_str("invalid").is_err());
    }

    #[test]
    fn extract_branch_prefix_works() {
        let content = r#"
layer: observers
branch_prefix: jules-observer-
constraints:
  - Do NOT write to issues/
"#;
        assert_eq!(extract_branch_prefix(content), Some("jules-observer-".to_string()));
    }
}
