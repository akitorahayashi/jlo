//! Workflow `pr sync-category-label` command implementation.
//!
//! Applies category label to implementer PRs by reading the label from the
//! PR head branch name and syncing it from `.jules/github-labels.json`.

use std::fs;
use std::path::Path;

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

/// Parsed implementer branch info.
struct ParsedBranch {
    label: String,
    #[allow(dead_code)]
    issue_id: String,
}

/// Execute `pr sync-category-label`.
pub fn execute(
    github: &impl GitHubPort,
    options: SyncCategoryLabelOptions,
) -> Result<SyncCategoryLabelOutput, AppError> {
    let pr = github.get_pr_detail(options.pr_number)?;

    // Only target implementer branches
    let parsed = match parse_implementer_branch(&pr.head) {
        Ok(p) => p,
        Err(_) => {
            return Ok(SyncCategoryLabelOutput {
                schema_version: 1,
                applied: false,
                skipped_reason: Some(format!(
                    "head branch '{}' does not match implementer pattern",
                    pr.head
                )),
                target: options.pr_number,
                label: None,
            });
        }
    };

    // Load label color from github-labels.json
    let labels_path = Path::new(".jules/github-labels.json");
    let label_info = load_label_info(labels_path, &parsed.label)?;

    // Ensure label exists with configured color, then apply to PR
    github.ensure_label(&label_info.name, Some(&label_info.color))?;
    github.add_label_to_pr(options.pr_number, &label_info.name)?;

    Ok(SyncCategoryLabelOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        target: options.pr_number,
        label: Some(label_info.name),
    })
}

/// Parse implementer branch name.
/// Expected format: `jules-implementer-<label>-<issue_id>` where issue_id is 6 alphanumeric chars.
fn parse_implementer_branch(branch: &str) -> Result<ParsedBranch, AppError> {
    if !branch.starts_with("jules-implementer-") {
        return Err(AppError::Validation(format!(
            "Branch '{}' does not match implementer pattern",
            branch
        )));
    }

    let suffix = branch.trim_start_matches("jules-implementer-");
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
            "Invalid implementer branch: issue ID must be 6 alphanumeric chars, got '{}' in '{}'",
            issue_id, branch
        )));
    }

    Ok(ParsedBranch { label: label.to_string(), issue_id: issue_id.to_string() })
}

/// Label information from github-labels.json.
struct LabelInfo {
    name: String,
    color: String,
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
    fn reject_non_implementer_branch() {
        assert!(parse_implementer_branch("jules-narrator-abc123").is_err());
        assert!(parse_implementer_branch("main").is_err());
    }

    #[test]
    fn reject_invalid_issue_id() {
        assert!(parse_implementer_branch("jules-implementer-bugs-abc").is_err());
        assert!(parse_implementer_branch("jules-implementer-bugs-abc1234").is_err());
        assert!(parse_implementer_branch("jules-implementer-bugs-ABC123").is_err());
    }
}
