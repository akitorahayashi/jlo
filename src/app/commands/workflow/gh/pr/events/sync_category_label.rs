//! Workflow `pr sync-category-label` command implementation.
//!
//! Applies category label to implementer PRs by reading the label from the
//! PR head branch name and syncing it from `.jules/github-labels.json`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHub;

/// Options for `workflow gh pr sync-category-label`.
#[derive(Debug, Clone)]
pub struct SyncCategoryLabelOptions {
    /// PR number to label.
    pub pr_number: u64,
}

/// Output of `workflow gh pr sync-category-label`.
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
    github: &impl GitHub,
    options: SyncCategoryLabelOptions,
) -> Result<SyncCategoryLabelOutput, AppError> {
    let pr = github.get_pr_detail(options.pr_number)?;

    let labels_path = Path::new(".jules/github-labels.json");
    let issue_labels = load_issue_labels(labels_path)?;

    // Only target implementer branches
    let parsed = match parse_implementer_branch(&pr.head, &issue_labels) {
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

    let label_info = issue_labels.get(&parsed.label).ok_or_else(|| {
        AppError::Validation(format!(
            "Label '{}' not found in github-labels.json issue_labels",
            parsed.label
        ))
    })?;

    // Ensure label exists with configured color, then apply to PR
    github.ensure_label(&label_info.name, Some(&label_info.color))?;
    github.add_label_to_pr(options.pr_number, &label_info.name)?;

    Ok(SyncCategoryLabelOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        target: options.pr_number,
        label: Some(label_info.name.clone()),
    })
}

/// Parse implementer branch name.
/// Expected format: `jules-implementer-<label>-<issue_id>-<short_description>`
/// where:
/// - `<label>` must exist in github-labels.json issue_labels keys
/// - `<issue_id>` is 6 lowercase alphanumeric chars
/// - `<short_description>` is non-empty and may contain hyphens.
fn parse_implementer_branch(
    branch: &str,
    issue_labels: &HashMap<String, LabelInfo>,
) -> Result<ParsedBranch, AppError> {
    if !branch.starts_with("jules-implementer-") {
        return Err(AppError::Validation(format!(
            "Branch '{}' does not match implementer pattern",
            branch
        )));
    }

    let suffix =
        branch.strip_prefix("jules-implementer-").expect("branch prefix is validated above");
    let mut labels: Vec<&str> = issue_labels.keys().map(|label| label.as_str()).collect();
    labels.sort_by_key(|label| std::cmp::Reverse(label.len()));

    for label in labels {
        let label_prefix = format!("{}-", label);
        let Some(rest) = suffix.strip_prefix(&label_prefix) else {
            continue;
        };
        let mut parts = rest.splitn(2, '-');
        let issue_id = parts.next().unwrap_or("");
        let short_description = parts.next().unwrap_or("");
        if short_description.is_empty() {
            continue;
        }
        if issue_id.len() == 6
            && issue_id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Ok(ParsedBranch { label: label.to_string(), issue_id: issue_id.to_string() });
        }
    }

    Err(AppError::Validation(format!(
        "Branch '{}' does not match implementer pattern '<label>-<id>-<short_description>'",
        branch
    )))
}

/// Label information from github-labels.json.
struct LabelInfo {
    name: String,
    color: String,
}

/// Load and validate issue labels from github-labels.json.
fn load_issue_labels(labels_path: &Path) -> Result<HashMap<String, LabelInfo>, AppError> {
    let content = fs::read_to_string(labels_path).map_err(|_| {
        AppError::Validation(format!("Missing github-labels.json: {}", labels_path.display()))
    })?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        AppError::ParseError { what: "github-labels.json".to_string(), details: e.to_string() }
    })?;

    let issue_labels = json.get("issue_labels").and_then(|v| v.as_object()).ok_or_else(|| {
        AppError::Validation("github-labels.json missing issue_labels object".to_string())
    })?;

    issue_labels
        .iter()
        .map(|(name, value)| {
            let color = value
                .get("color")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AppError::Validation(format!(
                        "Label '{}' missing color in github-labels.json",
                        name
                    ))
                })?
                .to_string();
            Ok((name.to_string(), LabelInfo { name: name.to_string(), color }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_implementer_branch() {
        let mut labels = HashMap::new();
        labels.insert(
            "bugs".to_string(),
            LabelInfo { name: "bugs".to_string(), color: "d73a4a".to_string() },
        );
        let parsed =
            parse_implementer_branch("jules-implementer-bugs-abc123-fix-crash", &labels).unwrap();
        assert_eq!(parsed.label, "bugs");
        assert_eq!(parsed.issue_id, "abc123");
    }

    #[test]
    fn parse_implementer_branch_with_hyphenated_label() {
        let mut labels = HashMap::new();
        labels.insert(
            "tech-debt".to_string(),
            LabelInfo { name: "tech-debt".to_string(), color: "0055aa".to_string() },
        );
        let parsed =
            parse_implementer_branch("jules-implementer-tech-debt-def456-refactor-parser", &labels)
                .unwrap();
        assert_eq!(parsed.label, "tech-debt");
        assert_eq!(parsed.issue_id, "def456");
    }

    #[test]
    fn reject_non_implementer_branch() {
        let labels = HashMap::new();
        assert!(parse_implementer_branch("jules-narrator-abc123", &labels).is_err());
        assert!(parse_implementer_branch("main", &labels).is_err());
    }

    #[test]
    fn reject_invalid_issue_id() {
        let mut labels = HashMap::new();
        labels.insert(
            "bugs".to_string(),
            LabelInfo { name: "bugs".to_string(), color: "d73a4a".to_string() },
        );
        assert!(parse_implementer_branch("jules-implementer-bugs-abc-fix", &labels).is_err());
        assert!(parse_implementer_branch("jules-implementer-bugs-abc1234-fix", &labels).is_err());
        assert!(parse_implementer_branch("jules-implementer-bugs-ABC123-fix", &labels).is_err());
    }

    #[test]
    fn reject_unknown_label_in_branch() {
        let mut labels = HashMap::new();
        labels.insert(
            "bugs".to_string(),
            LabelInfo { name: "bugs".to_string(), color: "d73a4a".to_string() },
        );
        assert!(
            parse_implementer_branch("jules-implementer-tech-debt-abc123-fix-parser", &labels)
                .is_err()
        );
    }
}
