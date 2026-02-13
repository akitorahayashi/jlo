//! Workflow `pr process` pipeline command implementation.
//!
//! Runs event-level PR commands in configured order and emits per-step results.

use std::thread;
use std::time::Duration;

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

use super::events::{comment_summary_request, enable_automerge, sync_category_label};

const TRANSIENT_AUTOMERGE_ERROR_PATTERNS: &[&str] = &[
    "enablePullRequestAutoMerge",
    "mergePullRequest",
    "Base branch was modified",
    "Protected branch rules not configured",
];
const RETRY_FIRST_DELAYS_SECONDS: &[u64] = &[1, 2, 5];

/// Execution mode for `workflow gh pr process`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    /// Run all PR event steps.
    All,
    /// Run metadata steps only: comment-summary-request and sync-category-label.
    Metadata,
    /// Run merge-gating step only: enable-automerge.
    Automerge,
}

impl ProcessMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Metadata => "metadata",
            Self::Automerge => "automerge",
        }
    }
}

/// Options for `workflow gh pr process`.
#[derive(Debug, Clone)]
pub struct ProcessOptions {
    /// PR number to process.
    pub pr_number: u64,
    /// Execution mode.
    pub mode: ProcessMode,
    /// Whether to fail immediately when any step returns an error.
    pub fail_on_error: bool,
    /// Retry attempts for transient auto-merge enable failures.
    pub retry_attempts: u32,
    /// Delay between retry attempts.
    pub retry_delay_seconds: u64,
}

/// Per-step result inside the pipeline output.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessStepResult {
    pub command: String,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub attempts: u32,
}

/// Output of `workflow gh pr process`.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessOutput {
    pub schema_version: u32,
    pub target: u64,
    pub mode: String,
    pub had_errors: bool,
    pub steps: Vec<ProcessStepResult>,
}

/// Execute `pr process`.
pub fn execute(
    github: &impl GitHubPort,
    options: ProcessOptions,
) -> Result<ProcessOutput, AppError> {
    if options.retry_attempts == 0 {
        return Err(AppError::Validation("retry_attempts must be greater than zero".to_string()));
    }

    let planned_steps = match options.mode {
        ProcessMode::All => vec![
            ProcessStep::CommentSummaryRequest,
            ProcessStep::SyncCategoryLabel,
            ProcessStep::EnableAutomerge,
        ],
        ProcessMode::Metadata => {
            vec![ProcessStep::CommentSummaryRequest, ProcessStep::SyncCategoryLabel]
        }
        ProcessMode::Automerge => vec![ProcessStep::EnableAutomerge],
    };

    let mut had_errors = false;
    let mut steps = Vec::with_capacity(planned_steps.len());

    for step in planned_steps {
        let result = match step {
            ProcessStep::CommentSummaryRequest => {
                run_comment_summary_request(github, options.pr_number)
            }
            ProcessStep::SyncCategoryLabel => run_sync_category_label(github, options.pr_number),
            ProcessStep::EnableAutomerge => run_enable_automerge(
                github,
                options.pr_number,
                options.retry_attempts,
                options.retry_delay_seconds,
            ),
        };

        if result.error.is_some() {
            had_errors = true;
            if options.fail_on_error {
                return Err(AppError::Validation(format!(
                    "workflow gh pr process failed at '{}' for PR #{}: {}",
                    result.command,
                    options.pr_number,
                    result.error.as_deref().unwrap_or("unknown error")
                )));
            }
        }

        steps.push(result);
    }

    Ok(ProcessOutput {
        schema_version: 1,
        target: options.pr_number,
        mode: options.mode.label().to_string(),
        had_errors,
        steps,
    })
}

#[derive(Debug, Clone, Copy)]
enum ProcessStep {
    CommentSummaryRequest,
    SyncCategoryLabel,
    EnableAutomerge,
}

fn run_comment_summary_request(github: &impl GitHubPort, pr_number: u64) -> ProcessStepResult {
    let opts = comment_summary_request::CommentSummaryRequestOptions { pr_number };
    match comment_summary_request::execute(github, opts) {
        Ok(out) => ProcessStepResult {
            command: "comment-summary-request".to_string(),
            applied: out.applied,
            skipped_reason: out.skipped_reason,
            error: None,
            attempts: 1,
        },
        Err(e) => ProcessStepResult {
            command: "comment-summary-request".to_string(),
            applied: false,
            skipped_reason: None,
            error: Some(e.to_string()),
            attempts: 1,
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
            error: None,
            attempts: 1,
        },
        Err(e) => ProcessStepResult {
            command: "sync-category-label".to_string(),
            applied: false,
            skipped_reason: None,
            error: Some(e.to_string()),
            attempts: 1,
        },
    }
}

fn run_enable_automerge(
    github: &impl GitHubPort,
    pr_number: u64,
    retry_attempts: u32,
    retry_delay_seconds: u64,
) -> ProcessStepResult {
    let opts = enable_automerge::EnableAutomergeOptions { pr_number };

    for attempt in 1..=retry_attempts {
        match github.get_pr_detail(pr_number) {
            Ok(pr_detail) if pr_detail.auto_merge_enabled => {
                return ProcessStepResult {
                    command: "enable-automerge".to_string(),
                    applied: false,
                    skipped_reason: Some("auto-merge already enabled".to_string()),
                    error: None,
                    attempts: attempt,
                };
            }
            Ok(_) => {}
            Err(error) => {
                return ProcessStepResult {
                    command: "enable-automerge".to_string(),
                    applied: false,
                    skipped_reason: None,
                    error: Some(error.to_string()),
                    attempts: attempt,
                };
            }
        }

        match enable_automerge::execute(github, opts.clone()) {
            Ok(out) => {
                return ProcessStepResult {
                    command: "enable-automerge".to_string(),
                    applied: out.applied,
                    skipped_reason: out.skipped_reason,
                    error: None,
                    attempts: attempt,
                };
            }
            Err(e) => {
                if let Ok(pr_detail) = github.get_pr_detail(pr_number)
                    && pr_detail.auto_merge_enabled
                {
                    return ProcessStepResult {
                        command: "enable-automerge".to_string(),
                        applied: false,
                        skipped_reason: Some("auto-merge already enabled".to_string()),
                        error: None,
                        attempts: attempt,
                    };
                }

                if attempt < retry_attempts && is_transient_automerge_error(&e) {
                    let sleep_seconds = compute_retry_delay_seconds(attempt, retry_delay_seconds);
                    if sleep_seconds > 0 {
                        thread::sleep(Duration::from_secs(sleep_seconds));
                    }
                    continue;
                }

                return ProcessStepResult {
                    command: "enable-automerge".to_string(),
                    applied: false,
                    skipped_reason: None,
                    error: Some(e.to_string()),
                    attempts: attempt,
                };
            }
        }
    }

    ProcessStepResult {
        command: "enable-automerge".to_string(),
        applied: false,
        skipped_reason: None,
        error: Some("auto-merge retry loop ended unexpectedly".to_string()),
        attempts: retry_attempts,
    }
}

fn compute_retry_delay_seconds(attempt: u32, configured_delay_seconds: u64) -> u64 {
    if configured_delay_seconds == 0 {
        return 0;
    }

    let profile_delay = RETRY_FIRST_DELAYS_SECONDS
        .get((attempt.saturating_sub(1)) as usize)
        .copied()
        .unwrap_or_else(|| RETRY_FIRST_DELAYS_SECONDS[RETRY_FIRST_DELAYS_SECONDS.len() - 1]);
    profile_delay.min(configured_delay_seconds)
}

fn is_transient_automerge_error(error: &AppError) -> bool {
    let message = error.to_string();
    TRANSIENT_AUTOMERGE_ERROR_PATTERNS.iter().any(|pattern| message.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
    use std::cell::{Cell, RefCell};

    struct FakeGitHub {
        pr_detail: RefCell<PullRequestDetail>,
        files: Vec<String>,
        remaining_transient_automerge_failures: Cell<u32>,
        fatal_automerge_failure: Cell<bool>,
        set_automerge_enabled_on_first_error: Cell<bool>,
        enable_calls: Cell<u32>,
    }

    impl FakeGitHub {
        fn jules_runtime_pr() -> Self {
            Self {
                pr_detail: RefCell::new(PullRequestDetail {
                    number: 42,
                    head: "jules-observer-abc123".to_string(),
                    base: "jules".to_string(),
                    is_draft: false,
                    auto_merge_enabled: false,
                }),
                files: vec![".jules/exchange/events/pending/state.yml".to_string()],
                remaining_transient_automerge_failures: Cell::new(0),
                fatal_automerge_failure: Cell::new(false),
                set_automerge_enabled_on_first_error: Cell::new(false),
                enable_calls: Cell::new(0),
            }
        }

        fn with_transient_automerge_failures(count: u32) -> Self {
            let gh = Self::jules_runtime_pr();
            gh.remaining_transient_automerge_failures.set(count);
            gh
        }

        fn with_fatal_automerge_failure() -> Self {
            let gh = Self::jules_runtime_pr();
            gh.fatal_automerge_failure.set(true);
            gh
        }

        fn with_race_automerge_state_after_first_failure() -> Self {
            let gh = Self::jules_runtime_pr();
            gh.remaining_transient_automerge_failures.set(1);
            gh.set_automerge_enabled_on_first_error.set(true);
            gh
        }

        fn non_jules_pr() -> Self {
            Self {
                pr_detail: RefCell::new(PullRequestDetail {
                    number: 99,
                    head: "feature/foo".to_string(),
                    base: "main".to_string(),
                    is_draft: false,
                    auto_merge_enabled: false,
                }),
                files: vec!["src/main.rs".to_string()],
                remaining_transient_automerge_failures: Cell::new(0),
                fatal_automerge_failure: Cell::new(false),
                set_automerge_enabled_on_first_error: Cell::new(false),
                enable_calls: Cell::new(0),
            }
        }
    }

    impl GitHubPort for FakeGitHub {
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
            Ok(self.pr_detail.borrow().clone())
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
            self.enable_calls.set(self.enable_calls.get() + 1);
            if self.fatal_automerge_failure.get() {
                return Err(AppError::ExternalToolError {
                    tool: "gh".to_string(),
                    error: "gh command failed: GraphQL: Validation failed: Pull request is not in a mergeable state".to_string(),
                });
            }
            let remaining = self.remaining_transient_automerge_failures.get();
            if remaining > 0 {
                self.remaining_transient_automerge_failures.set(remaining - 1);
                if self.set_automerge_enabled_on_first_error.get() {
                    self.pr_detail.borrow_mut().auto_merge_enabled = true;
                    self.set_automerge_enabled_on_first_error.set(false);
                }
                return Err(AppError::ExternalToolError {
                    tool: "gh".to_string(),
                    error: "gh command failed: GraphQL: Base branch was modified. Review and try the merge again. (mergePullRequest)".to_string(),
                });
            }
            self.pr_detail.borrow_mut().auto_merge_enabled = true;
            Ok(())
        }

        fn list_pr_files(&self, _: u64) -> Result<Vec<String>, AppError> {
            Ok(self.files.clone())
        }
    }

    #[test]
    fn automerge_mode_runs_only_enable_step() {
        let gh = FakeGitHub::jules_runtime_pr();
        let out = execute(
            &gh,
            ProcessOptions {
                pr_number: 42,
                mode: ProcessMode::Automerge,
                fail_on_error: true,
                retry_attempts: 1,
                retry_delay_seconds: 0,
            },
        )
        .unwrap();

        assert_eq!(out.mode, "automerge");
        assert!(!out.had_errors);
        assert_eq!(out.steps.len(), 1);
        assert_eq!(out.steps[0].command, "enable-automerge");
    }

    #[test]
    fn retries_transient_automerge_errors() {
        let gh = FakeGitHub::with_transient_automerge_failures(2);
        let out = execute(
            &gh,
            ProcessOptions {
                pr_number: 42,
                mode: ProcessMode::Automerge,
                fail_on_error: true,
                retry_attempts: 3,
                retry_delay_seconds: 0,
            },
        )
        .unwrap();

        assert!(!out.had_errors);
        assert_eq!(out.steps[0].attempts, 3);
        assert_eq!(gh.enable_calls.get(), 3);
    }

    #[test]
    fn does_not_retry_non_transient_automerge_errors() {
        let gh = FakeGitHub::with_fatal_automerge_failure();
        let out = execute(
            &gh,
            ProcessOptions {
                pr_number: 42,
                mode: ProcessMode::Automerge,
                fail_on_error: false,
                retry_attempts: 3,
                retry_delay_seconds: 0,
            },
        )
        .unwrap();

        assert!(out.had_errors);
        assert_eq!(out.steps[0].attempts, 1);
        assert_eq!(gh.enable_calls.get(), 1);
        assert!(out.steps[0].error.as_deref().unwrap_or("").contains("mergeable state"));
    }

    #[test]
    fn recheck_recovers_when_automerge_is_enabled_by_race() {
        let gh = FakeGitHub::with_race_automerge_state_after_first_failure();
        let out = execute(
            &gh,
            ProcessOptions {
                pr_number: 42,
                mode: ProcessMode::Automerge,
                fail_on_error: true,
                retry_attempts: 3,
                retry_delay_seconds: 0,
            },
        )
        .unwrap();

        assert!(!out.had_errors);
        assert_eq!(gh.enable_calls.get(), 1);
        assert_eq!(out.steps[0].attempts, 1);
        assert_eq!(out.steps[0].skipped_reason.as_deref(), Some("auto-merge already enabled"));
        assert!(out.steps[0].error.is_none());
    }

    #[test]
    fn fail_on_error_returns_validation_error() {
        let gh = FakeGitHub::jules_runtime_pr();
        let err = execute(
            &gh,
            ProcessOptions {
                pr_number: 42,
                mode: ProcessMode::Metadata,
                fail_on_error: true,
                retry_attempts: 1,
                retry_delay_seconds: 0,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("sync-category-label"));
    }

    #[test]
    fn non_fail_mode_reports_step_error() {
        let gh = FakeGitHub::jules_runtime_pr();
        let out = execute(
            &gh,
            ProcessOptions {
                pr_number: 42,
                mode: ProcessMode::Metadata,
                fail_on_error: false,
                retry_attempts: 1,
                retry_delay_seconds: 0,
            },
        )
        .unwrap();

        assert!(out.had_errors);
        assert_eq!(out.steps.len(), 2);
        assert!(out.steps[1].error.is_some());
    }

    #[test]
    fn automerge_mode_skips_non_jules_branch() {
        let gh = FakeGitHub::non_jules_pr();
        let out = execute(
            &gh,
            ProcessOptions {
                pr_number: 99,
                mode: ProcessMode::Automerge,
                fail_on_error: true,
                retry_attempts: 1,
                retry_delay_seconds: 0,
            },
        )
        .unwrap();

        assert!(!out.had_errors);
        assert_eq!(out.steps.len(), 1);
        assert!(!out.steps[0].applied);
        assert!(out.steps[0].skipped_reason.as_deref().unwrap_or("").contains("does not match"));
    }

    #[test]
    fn retry_delay_profile_is_retry_first_and_bounded() {
        assert_eq!(compute_retry_delay_seconds(1, 10), 1);
        assert_eq!(compute_retry_delay_seconds(2, 10), 2);
        assert_eq!(compute_retry_delay_seconds(3, 10), 5);
        assert_eq!(compute_retry_delay_seconds(4, 10), 5);
        assert_eq!(compute_retry_delay_seconds(1, 2), 1);
        assert_eq!(compute_retry_delay_seconds(3, 2), 2);
        assert_eq!(compute_retry_delay_seconds(1, 0), 0);
    }
}
