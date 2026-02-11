//! Workflow `pr enable-automerge` command implementation.
//!
//! Evaluates auto-merge policy gates and enables auto-merge on eligible PRs.
//! Policy gates (all must pass):
//! - Head branch starts with a known Jules layer prefix
//! - All changed files are within `.jules/`
//! - PR is not a draft
//! - Auto-merge is not already enabled

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Allowed branch prefixes derived from the Layer model.
const ALLOWED_PREFIXES: &[&str] =
    &["jules-narrator-", "jules-observer-", "jules-decider-", "jules-planner-", "jules-innovator-"];

/// Options for `workflow pr enable-automerge`.
#[derive(Debug, Clone)]
pub struct EnableAutomergeOptions {
    /// PR number to enable auto-merge on.
    pub pr_number: u64,
}

/// Output of `workflow pr enable-automerge`.
#[derive(Debug, Clone, Serialize)]
pub struct EnableAutomergeOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    pub target: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automerge_state: Option<String>,
}

/// Execute `pr enable-automerge`.
pub fn execute(
    github: &impl GitHubPort,
    options: EnableAutomergeOptions,
) -> Result<EnableAutomergeOutput, AppError> {
    let pr = github.get_pr_detail(options.pr_number)?;

    // Gate 1: branch prefix
    let prefix_match = ALLOWED_PREFIXES.iter().any(|p| pr.head.starts_with(p));
    if !prefix_match {
        return Ok(skip(
            options.pr_number,
            format!("head branch '{}' does not match any allowed Jules prefix", pr.head),
        ));
    }

    // Gate 2: draft state
    if pr.is_draft {
        return Ok(skip(options.pr_number, "PR is a draft".to_string()));
    }

    // Gate 3: already enabled
    if pr.auto_merge_enabled {
        return Ok(EnableAutomergeOutput {
            schema_version: 1,
            applied: false,
            skipped_reason: Some("auto-merge already enabled".to_string()),
            target: options.pr_number,
            automerge_state: Some("already_enabled".to_string()),
        });
    }

    // Gate 4: scope check — all changed files must be within .jules/
    let files = github.list_pr_files(options.pr_number)?;
    let non_jules: Vec<&String> = files.iter().filter(|f| !f.starts_with(".jules/")).collect();
    if !non_jules.is_empty() {
        return Ok(skip(
            options.pr_number,
            format!(
                "PR modifies files outside .jules/: {}",
                non_jules.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ),
        ));
    }

    // All gates passed — enable auto-merge
    github.enable_automerge(options.pr_number)?;

    Ok(EnableAutomergeOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        target: options.pr_number,
        automerge_state: Some("enabled".to_string()),
    })
}

fn skip(pr_number: u64, reason: String) -> EnableAutomergeOutput {
    EnableAutomergeOutput {
        schema_version: 1,
        applied: false,
        skipped_reason: Some(reason),
        target: pr_number,
        automerge_state: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
    use std::cell::RefCell;

    struct FakeGitHub {
        pr_detail: PullRequestDetail,
        files: Vec<String>,
        automerge_called: RefCell<bool>,
    }

    impl FakeGitHub {
        fn eligible_pr() -> Self {
            Self {
                pr_detail: PullRequestDetail {
                    number: 1,
                    head: "jules-narrator-abc123".to_string(),
                    base: "jules".to_string(),
                    is_draft: false,
                    auto_merge_enabled: false,
                },
                files: vec![".jules/exchange/events/pending/state.yml".to_string()],
                automerge_called: RefCell::new(false),
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
            *self.automerge_called.borrow_mut() = true;
            Ok(())
        }
        fn list_pr_files(&self, _: u64) -> Result<Vec<String>, AppError> {
            Ok(self.files.clone())
        }
    }

    #[test]
    fn enables_automerge_on_eligible_pr() {
        let gh = FakeGitHub::eligible_pr();
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 1 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.automerge_state.as_deref(), Some("enabled"));
        assert!(*gh.automerge_called.borrow());
    }

    #[test]
    fn skips_non_jules_branch() {
        let mut gh = FakeGitHub::eligible_pr();
        gh.pr_detail.head = "feature/something".to_string();
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 1 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("does not match"));
    }

    #[test]
    fn skips_draft_pr() {
        let mut gh = FakeGitHub::eligible_pr();
        gh.pr_detail.is_draft = true;
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 1 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("draft"));
    }

    #[test]
    fn skips_already_enabled() {
        let mut gh = FakeGitHub::eligible_pr();
        gh.pr_detail.auto_merge_enabled = true;
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 1 }).unwrap();
        assert!(!out.applied);
        assert_eq!(out.automerge_state.as_deref(), Some("already_enabled"));
    }

    #[test]
    fn skips_out_of_scope_files() {
        let mut gh = FakeGitHub::eligible_pr();
        gh.files = vec![".jules/state.yml".to_string(), "src/main.rs".to_string()];
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 1 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("outside .jules/"));
    }
}
