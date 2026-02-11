//! Workflow `issue label-innovator` command implementation.
//!
//! Applies `innovator` and `innovator/<persona>` labels to proposal issues.
//! Label color policy: existing labels keep their repository color; new labels
//! are created without specifying color so GitHub assigns a random one.
//! No color registry file is introduced.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHubPort;

/// Options for `workflow issue label-innovator`.
#[derive(Debug, Clone)]
pub struct LabelInnovatorOptions {
    /// Issue number to label.
    pub issue_number: u64,
    /// Persona name (e.g., "scout", "architect").
    pub persona: String,
}

/// Output of `workflow issue label-innovator`.
#[derive(Debug, Clone, Serialize)]
pub struct LabelInnovatorOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    pub target: u64,
    pub labels: Vec<String>,
}

/// Execute `issue label-innovator`.
pub fn execute(
    github: &impl GitHubPort,
    options: LabelInnovatorOptions,
) -> Result<LabelInnovatorOutput, AppError> {
    let base_label = "innovator".to_string();
    let persona_label = format!("innovator/{}", options.persona);

    // Ensure both labels exist (no color specified → GitHub assigns random on first creation)
    github.ensure_label(&base_label, None)?;
    github.ensure_label(&persona_label, None)?;

    // Apply both labels to the issue
    github.add_label_to_issue(options.issue_number, &base_label)?;
    github.add_label_to_issue(options.issue_number, &persona_label)?;

    Ok(LabelInnovatorOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        target: options.issue_number,
        labels: vec![base_label, persona_label],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
    use std::cell::RefCell;

    struct FakeGitHub {
        ensured_labels: RefCell<Vec<String>>,
        applied_labels: RefCell<Vec<(u64, String)>>,
    }

    impl FakeGitHub {
        fn new() -> Self {
            Self {
                ensured_labels: RefCell::new(Vec::new()),
                applied_labels: RefCell::new(Vec::new()),
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
            Ok(PullRequestDetail {
                number: 1,
                head: String::new(),
                base: String::new(),
                is_draft: false,
                auto_merge_enabled: false,
            })
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
        fn ensure_label(&self, label: &str, _color: Option<&str>) -> Result<(), AppError> {
            self.ensured_labels.borrow_mut().push(label.to_string());
            Ok(())
        }
        fn add_label_to_pr(&self, _: u64, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn add_label_to_issue(&self, issue: u64, label: &str) -> Result<(), AppError> {
            self.applied_labels.borrow_mut().push((issue, label.to_string()));
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
    fn applies_innovator_labels() {
        let gh = FakeGitHub::new();
        let out =
            execute(&gh, LabelInnovatorOptions { issue_number: 42, persona: "scout".to_string() })
                .unwrap();

        assert!(out.applied);
        assert_eq!(out.labels, vec!["innovator", "innovator/scout"]);
        assert_eq!(gh.ensured_labels.borrow().len(), 2);
        assert_eq!(gh.applied_labels.borrow().len(), 2);
        assert_eq!(gh.applied_labels.borrow()[0], (42, "innovator".to_string()));
        assert_eq!(gh.applied_labels.borrow()[1], (42, "innovator/scout".to_string()));
    }

    #[test]
    fn ensures_labels_without_color() {
        let gh = FakeGitHub::new();
        execute(&gh, LabelInnovatorOptions { issue_number: 1, persona: "architect".to_string() })
            .unwrap();

        // ensure_label is called with None color (random assignment by GitHub)
        assert!(gh.ensured_labels.borrow().contains(&"innovator".to_string()));
        assert!(gh.ensured_labels.borrow().contains(&"innovator/architect".to_string()));
    }
}
