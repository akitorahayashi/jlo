//! Workflow pr label-from-branch command implementation.
//!
//! Applies category label to an implementer PR using branch naming.

use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Options for workflow pr label-from-branch command.
#[derive(Debug, Clone)]
pub struct WorkflowPrLabelOptions {
    /// Branch name (defaults to GITHUB_REF_NAME if not provided).
    pub branch: Option<String>,
}

/// Output of workflow pr label-from-branch command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowPrLabelOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Whether a label was applied.
    pub applied: bool,
    /// PR number that was labeled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
    /// Label that was applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Issue ID extracted from branch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
}

/// Execute pr label-from-branch command.
pub fn execute(options: WorkflowPrLabelOptions) -> Result<WorkflowPrLabelOutput, AppError> {
    // Require GH_TOKEN and GITHUB_REPOSITORY
    if std::env::var("GH_TOKEN").is_err() {
        return Err(AppError::Validation(
            "GH_TOKEN environment variable is required for pr label-from-branch".to_string(),
        ));
    }
    if std::env::var("GITHUB_REPOSITORY").is_err() {
        return Err(AppError::Validation(
            "GITHUB_REPOSITORY environment variable is required for pr label-from-branch"
                .to_string(),
        ));
    }

    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Get branch name from option or env var
    let branch =
        options.branch.or_else(|| std::env::var("GITHUB_REF_NAME").ok()).ok_or_else(|| {
            AppError::Validation(
                "Branch name required: provide --branch or set GITHUB_REF_NAME".to_string(),
            )
        })?;

    // Parse branch name to extract label category
    // Expected format: jules-implementer-<label>-<issue_id>
    let parsed = parse_implementer_branch(&branch)?;

    // Load github-labels.json to validate label exists and get color
    let labels_path = workspace.jules_path().join("github-labels.json");
    let _label_info = load_label_info(&labels_path, &parsed.label)?;

    // For now, this is a stub implementation
    // Real implementation would:
    // 1. Find PR by head branch
    // 2. Ensure label exists on repo (create if needed with correct color)
    // 3. Apply label to PR

    eprintln!(
        "Would apply label '{}' to PR for branch '{}' (issue: {})",
        parsed.label, branch, parsed.issue_id
    );

    Ok(WorkflowPrLabelOutput {
        schema_version: 1,
        applied: false, // Would be true after actual GitHub API call
        pr_number: None,
        label: Some(parsed.label),
        issue_id: Some(parsed.issue_id),
    })
}

/// Parsed implementer branch info.
struct ParsedBranch {
    label: String,
    issue_id: String,
}

/// Parse implementer branch name.
/// Expected format: jules-implementer-<label>-<issue_id>
fn parse_implementer_branch(branch: &str) -> Result<ParsedBranch, AppError> {
    // Match pattern: jules-implementer-<label>-<issue_id>
    // The issue_id is the last segment (6 alphanumeric chars)
    // The label is everything between jules-implementer- and the issue_id

    if !branch.starts_with("jules-implementer-") {
        return Err(AppError::Validation(format!(
            "Branch '{}' does not match implementer pattern 'jules-implementer-<label>-<id>'",
            branch
        )));
    }

    let suffix = branch.trim_start_matches("jules-implementer-");

    // Find last hyphen to split label from issue_id
    let last_hyphen = suffix.rfind('-').ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid implementer branch format: missing issue ID in '{}'",
            branch
        ))
    })?;

    let label = &suffix[..last_hyphen];
    let issue_id = &suffix[last_hyphen + 1..];

    if label.is_empty() {
        return Err(AppError::Validation(format!(
            "Invalid implementer branch format: empty label in '{}'",
            branch
        )));
    }

    if issue_id.len() != 6
        || !issue_id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(AppError::Validation(format!(
            "Invalid implementer branch format: issue ID must be 6 alphanumeric chars, got '{}' in '{}'",
            issue_id, branch
        )));
    }

    Ok(ParsedBranch { label: label.to_string(), issue_id: issue_id.to_string() })
}

/// Load and validate label from github-labels.json.
fn load_label_info(labels_path: &Path, label: &str) -> Result<LabelInfo, AppError> {
    let content = fs::read_to_string(labels_path).map_err(|_| {
        AppError::Validation(format!("Missing github-labels.json: {}", labels_path.display()))
    })?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        AppError::ParseError { what: "github-labels.json".to_string(), details: e.to_string() }
    })?;

    let issue_labels = json.get("issue_labels").and_then(|v| v.as_object()).ok_or_else(|| {
        AppError::Validation("github-labels.json missing issue_labels object".to_string())
    })?;

    let label_obj = issue_labels.get(label).ok_or_else(|| {
        AppError::Validation(format!(
            "Label '{}' not found in github-labels.json issue_labels",
            label
        ))
    })?;

    let color = label_obj
        .get("color")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::Validation(format!("Label '{}' missing color in github-labels.json", label))
        })?
        .to_string();

    Ok(LabelInfo { name: label.to_string(), color })
}

/// Label information from github-labels.json.
#[allow(dead_code)]
struct LabelInfo {
    name: String,
    color: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_implementer_branch() {
        let parsed = parse_implementer_branch("jules-implementer-bugs-abc123").unwrap();
        assert_eq!(parsed.label, "bugs");
        assert_eq!(parsed.issue_id, "abc123");
    }

    #[test]
    fn parse_implementer_branch_with_hyphenated_label() {
        let parsed = parse_implementer_branch("jules-implementer-tech-debt-def456").unwrap();
        assert_eq!(parsed.label, "tech-debt");
        assert_eq!(parsed.issue_id, "def456");
    }

    #[test]
    fn reject_invalid_branch_pattern() {
        assert!(parse_implementer_branch("jules-observer-abc123").is_err());
        assert!(parse_implementer_branch("main").is_err());
    }

    #[test]
    fn reject_invalid_issue_id() {
        // Too short
        assert!(parse_implementer_branch("jules-implementer-bugs-abc").is_err());
        // Too long
        assert!(parse_implementer_branch("jules-implementer-bugs-abc1234").is_err());
        // Invalid chars
        assert!(parse_implementer_branch("jules-implementer-bugs-ABC123").is_err());
    }
}
