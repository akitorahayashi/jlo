//! Publish innovator proposals as GitHub issues.
//!
//! Scans all innovator rooms for merged `proposal.yml` files,
//! creates a GitHub issue from each proposal, and removes the proposal artifact
//! to mark publication as complete.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters::git_command::GitCommandAdapter;
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::domain::AppError;
use crate::ports::{GitHubPort, GitPort, IssueInfo, WorkspaceStore};

#[derive(Debug, Clone)]
pub struct WorkflowExchangePublishProposalsOptions {}

#[derive(Debug, Serialize)]
pub struct WorkflowExchangePublishProposalsOutput {
    pub schema_version: u32,
    pub published: Vec<PublishedProposal>,
    pub committed: bool,
    pub pushed: bool,
}

#[derive(Debug, Serialize)]
pub struct PublishedProposal {
    pub persona: String,
    pub proposal_path: String,
    pub issue_number: u64,
    pub issue_url: String,
}

/// Minimal deserialization of proposal.yml for issue creation.
#[derive(Debug, Deserialize)]
struct ProposalData {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    problem: String,
    #[serde(default)]
    introduction: String,
    #[serde(default)]
    importance: String,
    #[serde(default)]
    impact_surface: Vec<String>,
    #[serde(default)]
    implementation_cost: String,
    #[serde(default)]
    consistency_risks: Vec<String>,
    #[serde(default)]
    verification_signals: Vec<String>,
}

/// Minimal deserialization of perspective.yml for validation.
#[derive(Debug, Deserialize)]
struct PerspectiveData {
    #[serde(default)]
    recent_proposals: Vec<String>,
}

pub fn execute(
    options: WorkflowExchangePublishProposalsOptions,
) -> Result<WorkflowExchangePublishProposalsOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = workspace.jules_path();
    let root = jules_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let git = GitCommandAdapter::new(root.canonicalize().map_err(|e| {
        AppError::InternalError(format!("Failed to resolve workspace root: {}", e))
    })?);
    let github = crate::adapters::github_command::GitHubCommandAdapter::new();

    execute_with(&workspace, &options, &git, &github)
}

/// Core logic, injectable for testing.
fn execute_with<W, G, H>(
    workspace: &W,
    _options: &WorkflowExchangePublishProposalsOptions,
    git: &G,
    github: &H,
) -> Result<WorkflowExchangePublishProposalsOutput, AppError>
where
    W: WorkspaceStore,
    G: GitPort,
    H: GitHubPort,
{
    let jules_path = workspace.jules_path();
    let innovators_dir = jules_path.join("exchange").join("innovators");

    let proposals = discover_proposals(&innovators_dir, workspace)?;

    if proposals.is_empty() {
        return Ok(WorkflowExchangePublishProposalsOutput {
            schema_version: 1,
            published: vec![],
            committed: false,
            pushed: false,
        });
    }

    // Pass 1: Validate all proposals before any side-effects (issue creation).
    // This prevents partial failure leaving orphaned issues on GitHub.
    let mut validated: Vec<(String, PathBuf, String, String)> = Vec::new();
    for (persona, proposal_path) in &proposals {
        let content = workspace.read_file(
            proposal_path
                .to_str()
                .ok_or_else(|| AppError::Validation("Invalid proposal path".to_string()))?,
        )?;

        let data: ProposalData = serde_yaml::from_str(&content).map_err(|e| {
            AppError::Validation(format!(
                "Invalid YAML in proposal {}: {}",
                proposal_path.display(),
                e
            ))
        })?;

        if data.title.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "Proposal missing title: {}",
                proposal_path.display()
            )));
        }

        let required_fields = vec![
            ("problem", data.problem.trim().is_empty()),
            ("introduction", data.introduction.trim().is_empty()),
            ("importance", data.importance.trim().is_empty()),
            ("implementation_cost", data.implementation_cost.trim().is_empty()),
            ("impact_surface", data.impact_surface.is_empty()),
            ("consistency_risks", data.consistency_risks.is_empty()),
            ("verification_signals", data.verification_signals.is_empty()),
        ];

        for (field_name, is_missing) in required_fields {
            if is_missing {
                return Err(AppError::Validation(format!(
                    "Proposal missing '{}': {}",
                    field_name,
                    proposal_path.display()
                )));
            }
        }

        // Verify perspective.yml exists and records this proposal
        let perspective_path =
            proposal_path.parent().unwrap_or(Path::new(".")).join("perspective.yml");
        let perspective_path_str = perspective_path
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid perspective path".to_string()))?;
        if !workspace.file_exists(perspective_path_str) {
            return Err(AppError::Validation(format!(
                "perspective.yml missing for persona '{}': refinement must update perspective before publication",
                persona
            )));
        }
        let perspective_content = workspace.read_file(perspective_path_str)?;
        let perspective: PerspectiveData =
            serde_yaml::from_str(&perspective_content).map_err(|e| {
                AppError::Validation(format!(
                    "Invalid YAML in perspective {}: {}",
                    perspective_path.display(),
                    e
                ))
            })?;
        let title_trimmed = data.title.trim();
        if !perspective.recent_proposals.iter().any(|p| p.trim() == title_trimmed) {
            return Err(AppError::Validation(format!(
                "perspective.yml for '{}' does not list proposal '{}' in recent_proposals: refinement contract violated",
                persona, title_trimmed
            )));
        }

        let issue_title = format!("[innovator/{}] {}", persona, data.title.trim());
        let impact_surface = render_list(&data.impact_surface);
        let consistency_risks = render_list(&data.consistency_risks);
        let verification_signals = render_list(&data.verification_signals);

        let issue_body = format!(
            "## Problem\n\n{}\n\n## Introduction\n\n{}\n\n## Why It Matters\n\n{}\n\n## Impact Surface\n\n{}\n\n## Implementation Cost\n\n{}\n\n## Consistency Risks\n\n{}\n\n## Verification Signals\n\n{}\n\n---\n\n_Published from proposal `{}` by innovator persona `{}`._",
            data.problem.trim(),
            data.introduction.trim(),
            data.importance.trim(),
            impact_surface,
            data.implementation_cost.trim(),
            consistency_risks,
            verification_signals,
            data.id,
            persona,
        );

        validated.push((persona.clone(), proposal_path.clone(), issue_title, issue_body));
    }

    // Pass 2: Create issues and clean up artifacts (all proposals validated).
    let mut published = Vec::new();
    let mut deleted_paths: Vec<PathBuf> = Vec::new();

    for (persona, proposal_path, issue_title, issue_body) in &validated {
        let issue: IssueInfo = github.create_issue(issue_title, issue_body, &[])?;

        // Apply innovator labels to the newly created issue
        crate::app::commands::workflow::issue::label_innovator::execute(
            github,
            crate::app::commands::workflow::issue::LabelInnovatorOptions {
                issue_number: issue.number,
                persona: persona.clone(),
            },
        )?;

        published.push(PublishedProposal {
            persona: persona.clone(),
            proposal_path: proposal_path.display().to_string(),
            issue_number: issue.number,
            issue_url: issue.url.clone(),
        });

        // Remove proposal artifact
        workspace.remove_file(
            proposal_path
                .to_str()
                .ok_or_else(|| AppError::Validation("Invalid proposal path".to_string()))?,
        )?;
        deleted_paths.push(proposal_path.clone());

        // Clean comments directory if present
        let comments_dir = proposal_path.parent().unwrap_or(Path::new(".")).join("comments");
        if let Some(comments_dir_str) = comments_dir.to_str()
            && !comments_dir_str.is_empty()
            && let Ok(entries) = workspace.list_dir(comments_dir_str)
        {
            for entry in entries {
                if let Some(path_str) = entry.to_str() {
                    workspace.remove_file(path_str)?;
                    deleted_paths.push(entry);
                }
            }
        }
    }

    // Commit and push the deletions
    let files_refs: Vec<&Path> = deleted_paths.iter().map(|p| p.as_path()).collect();
    git.commit_files(
        &format!("jules: publish {} innovator proposal(s)", published.len()),
        &files_refs,
    )?;
    let branch = git.get_current_branch()?;
    git.push_branch(branch.trim(), false)?;

    Ok(WorkflowExchangePublishProposalsOutput {
        schema_version: 1,
        published,
        committed: true,
        pushed: true,
    })
}

/// Discover proposal.yml files across all innovator persona rooms.
fn discover_proposals<W: WorkspaceStore>(
    innovators_dir: &Path,
    workspace: &W,
) -> Result<Vec<(String, PathBuf)>, AppError> {
    let dir_str = innovators_dir
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid innovators path".to_string()))?;

    let persona_dirs = match workspace.list_dir(dir_str) {
        Ok(dirs) => dirs,
        Err(_) => return Ok(Vec::new()), // No innovators directory
    };

    let mut proposals = Vec::new();
    for persona_dir in persona_dirs {
        let Some(persona_dir_str) = persona_dir.to_str() else { continue };
        if !workspace.is_dir(persona_dir_str) {
            continue;
        }

        let Some(persona_name) = persona_dir.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if persona_name.is_empty() {
            continue;
        }

        let proposal_path = persona_dir.join("proposal.yml");
        let Some(proposal_str) = proposal_path.to_str() else { continue };
        if workspace.file_exists(proposal_str) {
            proposals.push((persona_name.to_string(), proposal_path));
        }
    }

    Ok(proposals)
}

fn render_list(items: &[String]) -> String {
    items.iter().map(|line| format!("- {}", line.trim())).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockWorkspaceStore;

    struct FakeGit;
    impl GitPort for FakeGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            Ok("abc123".to_string())
        }
        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok("jules".to_string())
        }
        fn commit_exists(&self, _sha: &str) -> bool {
            true
        }
        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
            Ok("abc000".to_string())
        }
        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            Ok(false)
        }
        fn run_command(&self, _args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
            Ok(String::new())
        }
        fn checkout_branch(&self, _branch: &str, _create: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn push_branch(&self, _branch: &str, _force: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            Ok("abc123".to_string())
        }
        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    struct FakeGitHub {
        created_issues: std::cell::RefCell<Vec<(String, String)>>,
    }

    impl FakeGitHub {
        fn new() -> Self {
            Self { created_issues: std::cell::RefCell::new(Vec::new()) }
        }
    }

    impl GitHubPort for FakeGitHub {
        fn dispatch_workflow(
            &self,
            _workflow_name: &str,
            _inputs: &[(&str, &str)],
        ) -> Result<(), AppError> {
            Ok(())
        }
        fn create_pull_request(
            &self,
            head: &str,
            base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<crate::ports::PullRequestInfo, AppError> {
            Ok(crate::ports::PullRequestInfo {
                number: 42,
                url: "https://example.com/pr/42".into(),
                head: head.to_string(),
                base: base.to_string(),
            })
        }
        fn close_pull_request(&self, _pr_number: u64) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_branch(&self, _branch: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn create_issue(
            &self,
            title: &str,
            body: &str,
            _labels: &[&str],
        ) -> Result<IssueInfo, AppError> {
            let count = self.created_issues.borrow().len() as u64 + 1;
            self.created_issues.borrow_mut().push((title.to_string(), body.to_string()));
            Ok(IssueInfo { number: count, url: format!("https://example.com/issues/{}", count) })
        }
        fn get_pr_detail(
            &self,
            _pr_number: u64,
        ) -> Result<crate::ports::PullRequestDetail, AppError> {
            Ok(crate::ports::PullRequestDetail {
                number: 42,
                head: String::new(),
                base: String::new(),
                is_draft: false,
                auto_merge_enabled: false,
            })
        }
        fn list_pr_comments(
            &self,
            _pr_number: u64,
        ) -> Result<Vec<crate::ports::PrComment>, AppError> {
            Ok(Vec::new())
        }
        fn create_pr_comment(&self, _pr_number: u64, _body: &str) -> Result<u64, AppError> {
            Ok(1)
        }
        fn update_pr_comment(&self, _comment_id: u64, _body: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn ensure_label(&self, _label: &str, _color: Option<&str>) -> Result<(), AppError> {
            Ok(())
        }
        fn add_label_to_pr(&self, _pr_number: u64, _label: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn add_label_to_issue(&self, _issue_number: u64, _label: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn enable_automerge(&self, _pr_number: u64) -> Result<(), AppError> {
            Ok(())
        }
        fn list_pr_files(&self, _pr_number: u64) -> Result<Vec<String>, AppError> {
            Ok(Vec::new())
        }
    }

    fn proposal_yaml() -> &'static str {
        r#"schema_version: 1
id: "abc123"
persona: "alice"
created_at: "2026-02-05"
title: "Improve error messages"
problem: |
  Error messages lack context.
introduction: |
  Introduce structured error narratives that preserve causal context.
importance: |
  Debugging feedback quality is currently limiting development speed.
impact_surface:
  - "Error reporting boundaries"
  - "Developer troubleshooting workflow"
implementation_cost: |
  Medium effort due to error path normalization and output updates.
consistency_risks:
  - "Mixed error style during adoption window"
verification_signals:
  - "Reduced ambiguity in reproduced failure reports"
"#
    }

    #[test]
    fn publishes_proposal_and_removes_artifact() {
        let proposal_path = ".jules/exchange/innovators/alice/proposal.yml";
        let perspective_path = ".jules/exchange/innovators/alice/perspective.yml";
        let perspective_yaml =
            "persona: alice\nrecent_proposals:\n  - \"Improve error messages\"\n";
        let workspace = MockWorkspaceStore::new()
            .with_exists(true)
            .with_file(proposal_path, proposal_yaml())
            .with_file(perspective_path, perspective_yaml);

        let git = FakeGit;
        let github = FakeGitHub::new();

        let options = WorkflowExchangePublishProposalsOptions {};

        let output = execute_with(&workspace, &options, &git, &github).unwrap();

        assert_eq!(output.published.len(), 1);
        assert_eq!(output.published[0].persona, "alice");
        assert_eq!(output.published[0].issue_number, 1);
        assert!(output.committed);
        assert!(output.pushed);

        // Proposal file should be removed
        assert!(!workspace.file_exists(proposal_path));

        // Verify issue was created with correct title
        let issues = github.created_issues.borrow();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].0.contains("[innovator/alice]"));
        assert!(issues[0].0.contains("Improve error messages"));
        assert!(issues[0].1.contains("## Why It Matters"));
        assert!(issues[0].1.contains("## Implementation Cost"));
        assert!(issues[0].1.contains("## Consistency Risks"));
    }

    #[test]
    fn no_proposals_returns_empty_output() {
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit;
        let github = FakeGitHub::new();

        let options = WorkflowExchangePublishProposalsOptions {};

        let output = execute_with(&workspace, &options, &git, &github).unwrap();

        assert!(output.published.is_empty());
        assert!(!output.committed);
        assert!(!output.pushed);
    }
}
