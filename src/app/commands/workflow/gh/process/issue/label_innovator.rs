//! Workflow `issue label-innovator` command implementation.
//!
//! Applies `innovator/<role>` label to proposal issues.
//! Label color policy: existing labels keep their repository color; new labels
//! are created without specifying color so GitHub assigns a random one.
//! No color registry file is introduced.

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHub;

/// Options for `workflow gh issue label-innovator`.
#[derive(Debug, Clone)]
pub struct LabelInnovatorOptions {
    /// Issue number to label.
    pub issue_number: u64,
    /// Role name (e.g., "scout", "architect").
    pub role: String,
}

/// Output of `workflow gh issue label-innovator`.
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
    github: &impl GitHub,
    options: LabelInnovatorOptions,
) -> Result<LabelInnovatorOutput, AppError> {
    let role_id = crate::domain::RoleId::new(&options.role)?;
    let role_label = format!("innovator/{}", role_id.as_str());

    // Ensure role label exists (no color specified → GitHub assigns random on first creation)
    github.ensure_label(&role_label, None)?;

    // Apply role label to the issue
    github.add_label_to_issue(options.issue_number, &role_label)?;

    Ok(LabelInnovatorOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        target: options.issue_number,
        labels: vec![role_label],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeGitHub;

    #[test]
    fn applies_only_role_innovator_label() {
        let gh = FakeGitHub::new();
        let out =
            execute(&gh, LabelInnovatorOptions { issue_number: 42, role: "scout".to_string() })
                .unwrap();

        assert!(out.applied);
        assert_eq!(out.labels, vec!["innovator/scout"]);
        assert_eq!(gh.ensured_labels.lock().unwrap().len(), 1);
        assert_eq!(gh.applied_labels.lock().unwrap().len(), 1);
        assert_eq!(gh.applied_labels.lock().unwrap()[0], (42, "innovator/scout".to_string()));
    }

    #[test]
    fn ensures_role_label_without_color() {
        let gh = FakeGitHub::new();
        execute(&gh, LabelInnovatorOptions { issue_number: 1, role: "architect".to_string() })
            .unwrap();

        // ensure_label is called with None color (random assignment by GitHub)
        assert!(gh.ensured_labels.lock().unwrap().contains(&"innovator/architect".to_string()));
    }
}
