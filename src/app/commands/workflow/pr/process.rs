//! Workflow `pr process` pipeline command implementation.
//!
//! Runs event-level PR commands in configured order and emits per-step results.
//! Pipeline order: comment-summary-request → sync-category-label → enable-automerge.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

use super::events::{comment_summary_request, enable_automerge, sync_category_label};

/// Options for `workflow pr process`.
#[derive(Debug, Clone)]
pub struct ProcessOptions {
    /// PR number to process.
    pub pr_number: u64,
}

/// Per-step result inside the pipeline output.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessStepResult {
    pub command: String,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

/// Output of `workflow pr process`.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessOutput {
    pub schema_version: u32,
    pub target: u64,
    pub steps: Vec<ProcessStepResult>,
}

/// Execute `pr process`.
///
/// Runs each event command in order, collecting results. A step failure
/// (Err) is recorded in the output; subsequent steps still execute.
pub fn execute(
    github: &impl GitHubPort,
    options: ProcessOptions,
) -> Result<ProcessOutput, AppError> {
    let pr = options.pr_number;

    let steps = vec![
        run_comment_summary_request(github, pr),
        run_sync_category_label(github, pr),
        run_enable_automerge(github, pr),
    ];

    Ok(ProcessOutput { schema_version: 1, target: pr, steps })
}

fn run_comment_summary_request(github: &impl GitHubPort, pr_number: u64) -> ProcessStepResult {
    let opts = comment_summary_request::CommentSummaryRequestOptions { pr_number };
    match comment_summary_request::execute(github, opts) {
        Ok(out) => ProcessStepResult {
            command: "comment-summary-request".to_string(),
            applied: out.applied,
            skipped_reason: out.skipped_reason,
        },
        Err(e) => ProcessStepResult {
            command: "comment-summary-request".to_string(),
            applied: false,
            skipped_reason: Some(format!("error: {e}")),
        },
    }
}

fn run_sync_category_label(github: &impl GitHubPort, pr_number: u64) -> ProcessStepResult {
    let opts = sync_category_label::SyncCategoryLabelOptions { pr_number };
    match sync_category_label::execute(github, opts) {
        Ok(out) => ProcessStepResult {
            command: "sync-category-label".to_string(),
            applied: out.applied,
            skipped_reason: out.skipped_reason,
        },
        Err(e) => ProcessStepResult {
            command: "sync-category-label".to_string(),
            applied: false,
            skipped_reason: Some(format!("error: {e}")),
        },
    }
}

fn run_enable_automerge(github: &impl GitHubPort, pr_number: u64) -> ProcessStepResult {
    let opts = enable_automerge::EnableAutomergeOptions { pr_number };
    match enable_automerge::execute(github, opts) {
        Ok(out) => ProcessStepResult {
            command: "enable-automerge".to_string(),
            applied: out.applied,
            skipped_reason: out.skipped_reason,
        },
        Err(e) => ProcessStepResult {
            command: "enable-automerge".to_string(),
            applied: false,
            skipped_reason: Some(format!("error: {e}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};

    struct FakeGitHub {
        pr_detail: PullRequestDetail,
        files: Vec<String>,
    }

    impl FakeGitHub {
        fn jules_pr() -> Self {
            Self {
                pr_detail: PullRequestDetail {
                    number: 42,
                    head: "jules-narrator-abc123".to_string(),
                    base: "jules".to_string(),
                    is_draft: false,
                    auto_merge_enabled: false,
                },
                files: vec![".jules/workstreams/generic/state.yml".to_string()],
            }
        }

        fn non_jules_pr() -> Self {
            Self {
                pr_detail: PullRequestDetail {
                    number: 99,
                    head: "feature/foo".to_string(),
                    base: "main".to_string(),
                    is_draft: false,
                    auto_merge_enabled: false,
                },
                files: vec!["src/main.rs".to_string()],
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
            Ok(Vec::new())
        }
        fn create_pr_comment(&self, _: u64, _: &str) -> Result<u64, AppError> {
            Ok(1)
        }
        fn update_pr_comment(&self, _: u64, _: &str) -> Result<(), AppError> {
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
            Ok(self.files.clone())
        }
    }

    #[test]
    fn process_runs_all_steps_on_jules_pr() {
        let gh = FakeGitHub::jules_pr();
        let out = execute(&gh, ProcessOptions { pr_number: 42 }).unwrap();
        assert_eq!(out.steps.len(), 3);
        assert_eq!(out.steps[0].command, "comment-summary-request");
        assert_eq!(out.steps[1].command, "sync-category-label");
        assert_eq!(out.steps[2].command, "enable-automerge");
        // comment-summary-request should apply on jules branch
        assert!(out.steps[0].applied);
        // sync-category-label should fail due to missing .jules/github-labels.json in test env
        assert!(!out.steps[1].applied);
        // enable-automerge should apply (all gates pass)
        assert!(out.steps[2].applied);
    }

    #[test]
    fn process_on_non_jules_pr_reports_skips() {
        let gh = FakeGitHub::non_jules_pr();
        let out = execute(&gh, ProcessOptions { pr_number: 99 }).unwrap();
        assert_eq!(out.steps.len(), 3);
        // comment-summary-request: skipped (not jules-* branch)
        assert!(!out.steps[0].applied);
        // enable-automerge: skipped (non-jules prefix)
        assert!(!out.steps[2].applied);
    }
}
