use std::process::Command;

use chrono::Utc;
use serde::Serialize;

use crate::adapters::git::GitCommandAdapter;
use crate::adapters::github::GitHubCommandAdapter;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::AppError;
use crate::ports::{Git, GitHub, JulesStore};

const WORKER_PUSH_BRANCH_PREFIX: &str = "jules-worker-sync-";

#[derive(Debug, Clone)]
pub struct PushWorkerBranchOptions {
    pub change_token: String,
    pub commit_message: String,
    pub pr_title: String,
    pub pr_body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PushWorkerBranchOutput {
    pub schema_version: u32,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
    pub merged: bool,
}

pub(crate) trait GhCliRunner {
    fn run(&self, args: &[&str]) -> Result<String, AppError>;
}

struct ShellGhCliRunner;

impl GhCliRunner for ShellGhCliRunner {
    fn run(&self, args: &[&str]) -> Result<String, AppError> {
        let output =
            Command::new("gh").args(args).output().map_err(|e| AppError::ExternalToolError {
                tool: "gh".to_string(),
                error: format!("Failed to execute gh CLI: {}", e),
            })?;

        if !output.status.success() {
            return Err(AppError::ExternalToolError {
                tool: "gh".to_string(),
                error: format!(
                    "gh command failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

pub fn execute(options: PushWorkerBranchOptions) -> Result<PushWorkerBranchOutput, AppError> {
    let repository = LocalRepositoryAdapter::current()?;
    if !repository.jules_exists() {
        return Err(AppError::JulesNotFound);
    }

    let root = repository
        .jules_path()
        .parent()
        .ok_or_else(|| AppError::Validation("Invalid .jules path: missing parent".to_string()))?
        .to_path_buf();
    let git = GitCommandAdapter::new(root);
    let github = GitHubCommandAdapter::new();
    let gh = ShellGhCliRunner;

    execute_with_adapters(&git, &github, &gh, options)
}

pub(crate) fn execute_with_adapters(
    git: &impl Git,
    github: &impl GitHub,
    gh: &impl GhCliRunner,
    options: PushWorkerBranchOptions,
) -> Result<PushWorkerBranchOutput, AppError> {
    validate_options(&options)?;

    let worker_branch = resolve_worker_branch_from_env()?;
    let current_branch = git.get_current_branch()?;
    if current_branch.trim() != worker_branch {
        return Err(AppError::Validation(format!(
            "workflow gh push worker-branch must run on '{}', current branch is '{}'",
            worker_branch, current_branch
        )));
    }

    let status = git.run_command(&["status", "--porcelain", "--", ".jules"], None)?;
    if status.trim().is_empty() {
        return Ok(PushWorkerBranchOutput {
            schema_version: 1,
            applied: false,
            skipped_reason: Some("No .jules changes to push".to_string()),
            branch: None,
            pr_number: None,
            head_sha: None,
            merged: false,
        });
    }

    let push_branch = build_worker_push_branch_name(&options.change_token);
    git.checkout_branch(&push_branch, true)?;

    git.run_command(&["add", "-A", "--", ".jules"], None)?;
    let staged = git.run_command(&["diff", "--cached", "--name-only"], None)?;
    if staged.trim().is_empty() {
        git.checkout_branch(&worker_branch, false)?;
        return Ok(PushWorkerBranchOutput {
            schema_version: 1,
            applied: false,
            skipped_reason: Some("No staged .jules changes to commit".to_string()),
            branch: None,
            pr_number: None,
            head_sha: None,
            merged: false,
        });
    }

    git.run_command(&["commit", "-m", &options.commit_message], None)?;
    let head_sha = git.get_head_sha()?;
    git.push_branch(&push_branch, false)?;

    let pr = github.create_pull_request(
        &push_branch,
        &worker_branch,
        &options.pr_title,
        &options.pr_body,
    )?;
    let pr_number = pr.number.to_string();
    gh.run(&["pr", "checks", &pr_number, "--watch"])?;
    gh.run(&["pr", "merge", &pr_number, "--squash", "--delete-branch"])?;

    git.fetch("origin")?;
    git.checkout_branch(&worker_branch, false)?;

    Ok(PushWorkerBranchOutput {
        schema_version: 1,
        applied: true,
        skipped_reason: None,
        branch: Some(push_branch),
        pr_number: Some(pr.number),
        head_sha: Some(head_sha),
        merged: true,
    })
}

fn validate_options(options: &PushWorkerBranchOptions) -> Result<(), AppError> {
    if options.change_token.trim().is_empty() {
        return Err(AppError::Validation("change_token is required".to_string()));
    }
    if options.commit_message.trim().is_empty() {
        return Err(AppError::Validation("commit_message is required".to_string()));
    }
    if options.pr_title.trim().is_empty() {
        return Err(AppError::Validation("pr_title is required".to_string()));
    }
    if options.pr_body.trim().is_empty() {
        return Err(AppError::Validation("pr_body is required".to_string()));
    }
    Ok(())
}

fn resolve_worker_branch_from_env() -> Result<String, AppError> {
    let branch = std::env::var("JULES_WORKER_BRANCH").map_err(|_| {
        AppError::Validation(
            "JULES_WORKER_BRANCH environment variable is required for worker-branch push"
                .to_string(),
        )
    })?;
    if branch.trim().is_empty() {
        return Err(AppError::Validation(
            "JULES_WORKER_BRANCH environment variable must not be empty".to_string(),
        ));
    }
    Ok(branch.trim().to_string())
}

fn build_worker_push_branch_name(change_token: &str) -> String {
    let token = sanitize_branch_segment(change_token);
    let ts = Utc::now().format("%Y%m%d%H%M%S");
    format!("{}{}-{}", WORKER_PUSH_BRANCH_PREFIX, token, ts)
}

fn sanitize_branch_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
            out.push(ch);
        } else if ch.is_ascii_uppercase() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }

    let collapsed =
        out.split('-').filter(|segment| !segment.is_empty()).collect::<Vec<_>>().join("-");
    if collapsed.is_empty() { "change".to_string() } else { collapsed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_branch_segment_normalizes_value() {
        assert_eq!(sanitize_branch_segment("Mock Cleanup/Run #1"), "mock-cleanup-run-1");
    }

    #[test]
    fn branch_name_uses_expected_prefix() {
        let branch = build_worker_push_branch_name("requirement-cleanup");
        assert!(branch.starts_with(WORKER_PUSH_BRANCH_PREFIX));
        assert!(branch.contains("requirement-cleanup-"));
    }
}
