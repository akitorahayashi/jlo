//! Workflow `pr comment-summary-request` command implementation.
//!
//! Posts or updates a managed bot comment on `jules-*` PRs requesting
//! the three-section summary template. Idempotent: updates existing
//! managed comment instead of duplicating.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHub;

/// Marker prefix embedded in the managed comment body for idempotent detection.
const MANAGED_COMMENT_MARKER: &str = "<!-- jlo:summary-request -->";

/// Summary request body for implementer PRs.
const IMPLEMENTER_SUMMARY_REQUEST_BODY: &str =
    include_str!("../../../../../../assets/summary-requests/implementer.md");

/// Summary request body for integrator PRs.
const INTEGRATOR_SUMMARY_REQUEST_BODY: &str =
    include_str!("../../../../../../assets/summary-requests/integrator.md");

/// Options for `workflow gh pr comment-summary-request`.
#[derive(Debug, Clone)]
pub struct CommentSummaryRequestOptions {
    /// PR number to comment on.
    pub pr_number: u64,
}

/// Output of `workflow gh pr comment-summary-request`.
#[derive(Debug, Clone, Serialize)]
pub struct CommentSummaryRequestOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    pub target: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_id: Option<u64>,
}

/// Select the appropriate summary request body based on branch prefix.
fn summary_request_body(head_branch: &str) -> &'static str {
    if head_branch.starts_with("jules-integrator-") {
        INTEGRATOR_SUMMARY_REQUEST_BODY
    } else {
        IMPLEMENTER_SUMMARY_REQUEST_BODY
    }
}

/// Execute `pr comment-summary-request`.
pub fn execute(
    github: &impl GitHub,
    options: CommentSummaryRequestOptions,
) -> Result<CommentSummaryRequestOutput, AppError> {
    let pr = github.get_pr_detail(options.pr_number)?;

    // Only target PRs whose head branch starts with `jules-`
    if !pr.head.starts_with("jules-") {
        return Ok(CommentSummaryRequestOutput {
            schema_version: 1,
            applied: false,
            skipped_reason: Some(format!("head branch '{}' does not start with 'jules-'", pr.head)),
            target: options.pr_number,
            comment_id: None,
        });
    }

    let body = summary_request_body(&pr.head);

    // Check for existing managed comment
    let comments = github.list_pr_comments(options.pr_number)?;
    let existing = comments.iter().find(|c| c.body.contains(MANAGED_COMMENT_MARKER));

    let comment_id = if let Some(managed) = existing {
        // Update existing comment
        github.update_pr_comment(managed.id, body)?;
        managed.id
    } else {
        // Create new comment
        github.create_pr_comment(options.pr_number, body)?
    };

    Ok(CommentSummaryRequestOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        target: options.pr_number,
        comment_id: Some(comment_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::PrComment;
    use crate::testing::FakeGitHub;

    #[test]
    fn skips_non_jules_branch() {
        let gh = FakeGitHub::non_jules_pr();
        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 99 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("does not start with 'jules-'"));
    }

    #[test]
    fn creates_comment_on_jules_pr() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().head = "jules-narrator-abc123".to_string();
        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.comment_id, Some(100));
        assert_eq!(gh.comments.lock().unwrap().len(), 1);
        assert!(gh.comments.lock().unwrap()[0].body.contains(MANAGED_COMMENT_MARKER));
    }

    #[test]
    fn updates_existing_managed_comment() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().head = "jules-narrator-abc123".to_string();
        // Seed an existing managed comment
        gh.comments
            .lock()
            .unwrap()
            .push(PrComment { id: 50, body: format!("{}\nold content", MANAGED_COMMENT_MARKER) });

        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.comment_id, Some(50));
        // Should update, not create a new one
        assert_eq!(gh.comments.lock().unwrap().len(), 1);
        assert!(gh.comments.lock().unwrap()[0].body.contains("Summary of Changes"));
    }

    #[test]
    fn implementer_pr_uses_implementer_template() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().head = "jules-implementer-bugs-abc123-fix-crash".to_string();
        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        let comments = gh.comments.lock().unwrap();
        let body = &comments[0].body;
        assert!(
            body.contains("Summary of Changes"),
            "implementer template should contain 'Summary of Changes'"
        );
        assert!(
            !body.contains("Integration Summary"),
            "implementer template should not contain integrator sections"
        );
    }

    #[test]
    fn integrator_pr_uses_integrator_template() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().head = "jules-integrator-20260213-abc123".to_string();
        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        let comments = gh.comments.lock().unwrap();
        let body = &comments[0].body;
        assert!(
            body.contains("Integration Summary"),
            "integrator template should contain 'Integration Summary'"
        );
        assert!(
            body.contains("Conflict Resolutions"),
            "integrator template should contain 'Conflict Resolutions'"
        );
        assert!(
            body.contains("Risk Assessment"),
            "integrator template should contain 'Risk Assessment'"
        );
    }
}
