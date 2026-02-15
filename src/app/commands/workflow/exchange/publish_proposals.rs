//! Publish innovator proposals as GitHub issues.
//!
//! Scans `.jules/exchange/proposals/*.yml`, creates a GitHub issue from each
//! proposal, and removes the proposal artifact to mark publication as complete.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters::git::GitCommandAdapter;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{Git, GitHub, IssueInfo, JloStore, JulesStore, RepositoryFilesystem};

#[derive(Debug, Clone)]
pub struct ExchangePublishProposalsOptions {}

#[derive(Debug, Serialize)]
pub struct ExchangePublishProposalsOutput {
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
    persona: String,
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
    options: ExchangePublishProposalsOptions,
) -> Result<ExchangePublishProposalsOutput, AppError> {
    let repository = LocalRepositoryAdapter::current()?;
    if !repository.jules_exists() {
        return Err(AppError::JulesNotFound);
    }

    let jules_path = repository.jules_path();
    let root = jules_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let git = GitCommandAdapter::new(root.canonicalize().map_err(|e| {
        AppError::InternalError(format!("Failed to resolve repository root: {}", e))
    })?);
    let github = crate::adapters::github::GitHubCommandAdapter::new();

    execute_with(&repository, &options, &git, &github)
}

/// Core logic, injectable for testing.
fn execute_with<W, G, H>(
    repository: &W,
    _options: &ExchangePublishProposalsOptions,
    git: &G,
    github: &H,
) -> Result<ExchangePublishProposalsOutput, AppError>
where
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
    G: Git,
    H: GitHub,
{
    let jules_path = repository.jules_path();
    let proposals_dir = crate::domain::exchange::proposals::paths::proposals_dir(&jules_path);

    let proposals = discover_proposals(&proposals_dir, repository)?;

    if proposals.is_empty() {
        return Ok(ExchangePublishProposalsOutput {
            schema_version: 1,
            published: vec![],
            committed: false,
            pushed: false,
        });
    }

    // Pass 1: Validate all proposals before any side-effects (issue creation).
    // This prevents partial failure leaving orphaned issues on GitHub.
    let mut validated: Vec<(String, PathBuf, String, String)> = Vec::new();
    for proposal_path in &proposals {
        let content = repository.read_file(
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

        let persona = data.persona.trim();
        if persona.is_empty() {
            return Err(AppError::Validation(format!(
                "Proposal missing 'persona': {}",
                proposal_path.display()
            )));
        }

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
            crate::domain::workstations::paths::workstation_perspective(&jules_path, persona);
        let perspective_path_str = perspective_path
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid perspective path".to_string()))?;
        if !repository.file_exists(perspective_path_str) {
            return Err(AppError::Validation(format!(
                "perspective.yml missing for persona '{}': innovator run must update workstation perspective before publication",
                persona
            )));
        }
        let perspective_content = repository.read_file(perspective_path_str)?;
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

        validated.push((persona.to_string(), proposal_path.clone(), issue_title, issue_body));
    }

    // Pass 2: Create issues and clean up artifacts (all proposals validated).
    let mut published = Vec::new();
    let mut deleted_paths: Vec<PathBuf> = Vec::new();

    for (persona, proposal_path, issue_title, issue_body) in &validated {
        let issue: IssueInfo = github.create_issue(issue_title, issue_body, &[])?;

        // Apply innovator labels to the newly created issue
        crate::app::commands::workflow::gh::issue::label_innovator::execute(
            github,
            crate::app::commands::workflow::gh::issue::LabelInnovatorOptions {
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
        repository.remove_file(
            proposal_path
                .to_str()
                .ok_or_else(|| AppError::Validation("Invalid proposal path".to_string()))?,
        )?;
        deleted_paths.push(proposal_path.clone());
    }

    // Commit and push the deletions
    let files_refs: Vec<&Path> = deleted_paths.iter().map(|p| p.as_path()).collect();
    git.commit_files(
        &format!("jules: publish {} innovator proposal(s)", published.len()),
        &files_refs,
    )?;
    let branch = git.get_current_branch()?;
    git.push_branch(branch.trim(), false)?;

    Ok(ExchangePublishProposalsOutput {
        schema_version: 1,
        published,
        committed: true,
        pushed: true,
    })
}

/// Discover proposal files under `.jules/exchange/proposals/`.
fn discover_proposals<W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader>(
    proposals_dir: &Path,
    repository: &W,
) -> Result<Vec<PathBuf>, AppError> {
    let dir_str = proposals_dir
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid proposals path".to_string()))?;

    let entries = match repository.list_dir(dir_str) {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()), // No proposals directory
    };

    let mut proposals = Vec::new();
    for path in entries {
        let Some(path_str) = path.to_str() else { continue };
        if repository.is_dir(path_str) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        proposals.push(path);
    }

    proposals.sort();
    Ok(proposals)
}

fn render_list(items: &[String]) -> String {
    items.iter().map(|line| format!("- {}", line.trim())).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RepositoryFilesystem;
    use crate::testing::{FakeGit, FakeGitHub, TestStore};

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
        let proposal_path = ".jules/exchange/proposals/alice-improve-error-messages.yml";
        let perspective_path = ".jules/workstations/alice/perspective.yml";
        let perspective_yaml =
            "persona: alice\nrecent_proposals:\n  - \"Improve error messages\"\n";
        let repository = TestStore::new()
            .with_exists(true)
            .with_file(proposal_path, proposal_yaml())
            .with_file(perspective_path, perspective_yaml);

        let git = FakeGit::new();
        let github = FakeGitHub::new();

        let options = ExchangePublishProposalsOptions {};

        let output = execute_with(&repository, &options, &git, &github).unwrap();

        assert_eq!(output.published.len(), 1);
        assert_eq!(output.published[0].persona, "alice");
        assert_eq!(output.published[0].issue_number, 1);
        assert!(output.committed);
        assert!(output.pushed);

        // Proposal file should be removed
        assert!(!repository.file_exists(proposal_path));

        // Verify issue was created with correct title
        let issues = github.created_issues.lock().unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].0.contains("[innovator/alice]"));
        assert!(issues[0].0.contains("Improve error messages"));
        assert!(issues[0].1.contains("## Why It Matters"));
        assert!(issues[0].1.contains("## Implementation Cost"));
        assert!(issues[0].1.contains("## Consistency Risks"));
    }

    #[test]
    fn no_proposals_returns_empty_output() {
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();

        let options = ExchangePublishProposalsOptions {};

        let output = execute_with(&repository, &options, &git, &github).unwrap();

        assert!(output.published.is_empty());
        assert!(!output.committed);
        assert!(!output.pushed);
    }
}
