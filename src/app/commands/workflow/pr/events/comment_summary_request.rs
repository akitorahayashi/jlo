//! Workflow `pr comment-summary-request` command implementation.
//!
//! Posts or updates a managed bot comment on `jules-*` PRs requesting
//! the three-section summary template. Idempotent: updates existing
//! managed comment instead of duplicating.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Marker prefix embedded in the managed comment body for idempotent detection.
const MANAGED_COMMENT_MARKER: &str = "<!-- jlo:summary-request -->";

/// Fixed summary request comment body per spec.
const SUMMARY_REQUEST_BODY: &str = include_str!("../../../../../assets/prompts/summary_request.md");

/// Options for `workflow pr comment-summary-request`.
#[derive(Debug, Clone)]
pub struct CommentSummaryRequestOptions {
    /// PR number to comment on.
    pub pr_number: u64,
}

/// Output of `workflow pr comment-summary-request`.
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

/// Execute `pr comment-summary-request`.
pub fn execute(
    github: &impl GitHubPort,
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

    // Check for existing managed comment
    let comments = github.list_pr_comments(options.pr_number)?;
    let existing = comments.iter().find(|c| c.body.contains(MANAGED_COMMENT_MARKER));

    let comment_id = if let Some(managed) = existing {
        // Update existing comment
        github.update_pr_comment(managed.id, SUMMARY_REQUEST_BODY)?;
        managed.id
    } else {
        // Create new comment
        github.create_pr_comment(options.pr_number, SUMMARY_REQUEST_BODY)?
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
    use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
    use std::cell::RefCell;

    struct FakeGitHub {
        pr_detail: PullRequestDetail,
        comments: RefCell<Vec<PrComment>>,
        next_comment_id: RefCell<u64>,
    }

    impl FakeGitHub {
        fn jules_pr() -> Self {
            Self {
                pr_detail: PullRequestDetail {
                    number: 1,
                    head: "jules-narrator-abc123".to_string(),
                    base: "main".to_string(),
                    is_draft: false,
                    auto_merge_enabled: false,
                },
                comments: RefCell::new(Vec::new()),
                next_comment_id: RefCell::new(100),
            }
        }

        fn non_jules_pr() -> Self {
            Self {
                pr_detail: PullRequestDetail {
                    number: 2,
                    head: "feature/something".to_string(),
                    base: "main".to_string(),
                    is_draft: false,
                    auto_merge_enabled: false,
                },
                comments: RefCell::new(Vec::new()),
                next_comment_id: RefCell::new(100),
            }
        }
    }

    impl GitHubPort for FakeGitHub {
        fn dispatch_workflow(&self, _: &str, _: &[(&str, &str)]) -> Result<(), AppError> {
            Ok(())
        }
        fn create_pull_request(
            &self,
            h: &str,
            b: &str,
            _: &str,
            _: &str,
        ) -> Result<PullRequestInfo, AppError> {
            Ok(PullRequestInfo { number: 1, url: String::new(), head: h.into(), base: b.into() })
        }
        fn close_pull_request(&self, _: u64) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_branch(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn create_issue(&self, _: &str, _: &str, _: &[&str]) -> Result<IssueInfo, AppError> {
            Ok(IssueInfo { number: 1, url: String::new() })
        }
        fn get_pr_detail(&self, _: u64) -> Result<PullRequestDetail, AppError> {
            Ok(self.pr_detail.clone())
        }
        fn list_pr_comments(&self, _: u64) -> Result<Vec<PrComment>, AppError> {
            Ok(self.comments.borrow().clone())
        }
        fn create_pr_comment(&self, _: u64, body: &str) -> Result<u64, AppError> {
            let id = *self.next_comment_id.borrow();
            *self.next_comment_id.borrow_mut() += 1;
            self.comments.borrow_mut().push(PrComment { id, body: body.to_string() });
            Ok(id)
        }
        fn update_pr_comment(&self, id: u64, body: &str) -> Result<(), AppError> {
            let mut comments = self.comments.borrow_mut();
            if let Some(c) = comments.iter_mut().find(|c| c.id == id) {
                c.body = body.to_string();
            }
            Ok(())
        }
        fn ensure_label(&self, _: &str, _: Option<&str>) -> Result<(), AppError> {
            Ok(())
        }
        fn add_label_to_pr(&self, _: u64, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn add_label_to_issue(&self, _: u64, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn enable_automerge(&self, _: u64) -> Result<(), AppError> {
            Ok(())
        }
        fn list_pr_files(&self, _: u64) -> Result<Vec<String>, AppError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn skips_non_jules_branch() {
        let gh = FakeGitHub::non_jules_pr();
        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 2 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("does not start with 'jules-'"));
    }

    #[test]
    fn creates_comment_on_jules_pr() {
        let gh = FakeGitHub::jules_pr();
        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 1 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.comment_id, Some(100));
        assert_eq!(gh.comments.borrow().len(), 1);
        assert!(gh.comments.borrow()[0].body.contains(MANAGED_COMMENT_MARKER));
    }

    #[test]
    fn updates_existing_managed_comment() {
        let gh = FakeGitHub::jules_pr();
        // Seed an existing managed comment
        gh.comments
            .borrow_mut()
            .push(PrComment { id: 50, body: format!("{}\nold content", MANAGED_COMMENT_MARKER) });

        let out = execute(&gh, CommentSummaryRequestOptions { pr_number: 1 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.comment_id, Some(50));
        // Should update, not create a new one
        assert_eq!(gh.comments.borrow().len(), 1);
        assert!(gh.comments.borrow()[0].body.contains("Summary of Changes"));
    }
}
