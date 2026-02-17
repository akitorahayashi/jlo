//! Workflow `pr enable-automerge` command implementation.
//!
//! Evaluates auto-merge policy gates and enables auto-merge on eligible PRs.
//! Policy gates (all must pass):
//! - Head branch starts with a known Jules layer prefix
//! - Changed-file scope matches the branch policy
//! - PR is not a draft
//! - Auto-merge is not already enabled

use serde::Serialize;

use crate::domain::AppError;
use crate::ports::GitHub;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopePolicy {
    JulesOnly,
    RepositoryWide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BranchPolicy {
    prefix: &'static str,
    scope: ScopePolicy,
}

/// Allowed branch prefixes and their change-scope policy.
const BRANCH_POLICIES: &[BranchPolicy] = &[
    BranchPolicy { prefix: "jules-narrator-", scope: ScopePolicy::JulesOnly },
    BranchPolicy { prefix: "jules-observer-", scope: ScopePolicy::JulesOnly },
    BranchPolicy { prefix: "jules-decider-", scope: ScopePolicy::JulesOnly },
    BranchPolicy { prefix: "jules-planner-", scope: ScopePolicy::JulesOnly },
    BranchPolicy { prefix: "jules-innovator-", scope: ScopePolicy::JulesOnly },
    BranchPolicy { prefix: "jules-publish-proposals-", scope: ScopePolicy::JulesOnly },
    BranchPolicy { prefix: "jules-mock-cleanup-", scope: ScopePolicy::JulesOnly },
    // Bootstrap branches carry target->worker sync changes and runtime materialization updates.
    BranchPolicy { prefix: "jules-bootstrap-", scope: ScopePolicy::RepositoryWide },
];

/// Options for `workflow gh pr enable-automerge`.
#[derive(Debug, Clone)]
pub struct EnableAutomergeOptions {
    /// PR number to enable auto-merge on.
    pub pr_number: u64,
}

/// Output of `workflow gh pr enable-automerge`.
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
    github: &impl GitHub,
    options: EnableAutomergeOptions,
) -> Result<EnableAutomergeOutput, AppError> {
    let pr = github.get_pr_detail(options.pr_number)?;

    // Gate 1: branch prefix
    let branch_policy = match branch_policy_for_head(&pr.head) {
        Some(policy) => policy,
        None => {
            return Ok(skip(
                options.pr_number,
                format!("head branch '{}' does not match any allowed Jules prefix", pr.head),
            ));
        }
    };

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

    // Gate 4: scope policy check
    let files = github.list_pr_files(options.pr_number)?;
    if branch_policy.scope == ScopePolicy::JulesOnly {
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

fn branch_policy_for_head(head: &str) -> Option<&'static BranchPolicy> {
    BRANCH_POLICIES.iter().find(|policy| head.starts_with(policy.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeGitHub;
    use std::sync::atomic::Ordering;

    #[test]
    fn enables_automerge_on_eligible_pr() {
        let gh = FakeGitHub::jules_runtime_pr();
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.automerge_state.as_deref(), Some("enabled"));
        assert!(gh.automerge_calls.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn enables_automerge_on_mock_cleanup_branch() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().head = "jules-mock-cleanup-mock-run-123".to_string();
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.automerge_state.as_deref(), Some("enabled"));
        assert!(gh.automerge_calls.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn enables_automerge_on_publish_proposals_branch() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().head = "jules-publish-proposals-20260215120000".to_string();
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.automerge_state.as_deref(), Some("enabled"));
        assert!(gh.automerge_calls.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn enables_automerge_on_bootstrap_branch_with_repository_scope() {
        let gh = FakeGitHub::jules_runtime_pr().with_files(vec![
            "src/main.rs".to_string(),
            ".github/workflows/jules-scheduled-workflows.yml".to_string(),
            ".jules/README.md".to_string(),
        ]);
        gh.pr_detail.lock().unwrap().head = "jules-bootstrap-12345-1".to_string();
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(out.applied);
        assert_eq!(out.automerge_state.as_deref(), Some("enabled"));
        assert!(gh.automerge_calls.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn skips_non_jules_branch() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().head = "feature/something".to_string();
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("does not match"));
    }

    #[test]
    fn skips_draft_pr() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().is_draft = true;
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("draft"));
    }

    #[test]
    fn skips_already_enabled() {
        let gh = FakeGitHub::jules_runtime_pr();
        gh.pr_detail.lock().unwrap().auto_merge_enabled = true;
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(!out.applied);
        assert_eq!(out.automerge_state.as_deref(), Some("already_enabled"));
    }

    #[test]
    fn skips_out_of_scope_files() {
        let gh = FakeGitHub::jules_runtime_pr()
            .with_files(vec![".jules/state.yml".to_string(), "src/main.rs".to_string()]);
        let out = execute(&gh, EnableAutomergeOptions { pr_number: 42 }).unwrap();
        assert!(!out.applied);
        assert!(out.skipped_reason.unwrap().contains("outside .jules/"));
    }
}
