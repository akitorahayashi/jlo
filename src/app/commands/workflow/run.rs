//! Workflow run command implementation.
//!
//! Executes a layer via the existing run infrastructure and returns wait-gating metadata.
//! This command provides orchestration metadata for GitHub Actions workflows.

use chrono::Utc;
use serde::Serialize;
use std::path::PathBuf;

use crate::app::commands::run::{self, RunOptions, RunResult};
use crate::domain::{AppError, Layer};
use crate::ports::WorkspaceStore;
use crate::services::adapters::git_cli::CliGit;
use crate::services::adapters::github_cli::CliGitHub;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

/// Options for workflow run command.
#[derive(Debug, Clone)]
pub struct WorkflowRunOptions {
    /// Target layer.
    pub layer: Layer,
    /// Matrix JSON input (required for non-narrator layers).
    pub matrix_json: Option<serde_json::Value>,
    /// Target branch for implementers.
    #[allow(dead_code)]
    pub target_branch: Option<String>,
    /// Run in mock mode.
    pub mock: bool,
}

/// Output of workflow run command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRunOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Timestamp when run started (RFC3339 UTC).
    pub run_started_at: String,
    /// Mock tag (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_tag: Option<String>,
    /// Mock PR numbers (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_pr_numbers: Option<Vec<u64>>,
    /// Mock branches (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_branches: Option<Vec<String>>,
}

/// Execute workflow run command.
pub fn execute(options: WorkflowRunOptions) -> Result<WorkflowRunOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Validate matrix is provided for layers that need it
    validate_matrix_requirement(&options)?;

    let run_started_at = Utc::now().to_rfc3339();

    // Mock mode configuration
    let mock_tag = if options.mock {
        let tag = std::env::var("JULES_MOCK_TAG").map_err(|_| {
            AppError::Validation(
                "Mock mode requires JULES_MOCK_TAG environment variable".to_string(),
            )
        })?;

        if !tag.contains("mock") {
            return Err(AppError::Validation(
                "JULES_MOCK_TAG must contain 'mock' substring".to_string(),
            ));
        }
        Some(tag)
    } else {
        None
    };

    // Execute layer runs using existing run infrastructure
    let run_results = execute_layer(&options, &workspace)?;

    Ok(WorkflowRunOutput {
        schema_version: 1,
        run_started_at,
        mock_tag,
        mock_pr_numbers: run_results.mock_pr_numbers,
        mock_branches: run_results.mock_branches,
    })
}

/// Results from running a layer.
struct RunResults {
    mock_pr_numbers: Option<Vec<u64>>,
    mock_branches: Option<Vec<String>>,
}

/// Execute runs for a layer using existing run infrastructure.
fn execute_layer(
    options: &WorkflowRunOptions,
    workspace: &FilesystemWorkspaceStore,
) -> Result<RunResults, AppError> {
    let git = CliGit;
    let github = CliGitHub;
    let jules_path = workspace.jules_path();

    match options.layer {
        Layer::Narrators => execute_narrator(options.mock, &jules_path, &git, &github, workspace),
        Layer::Observers => execute_observers(options, &jules_path, &git, &github, workspace),
        Layer::Deciders => execute_deciders(options, &jules_path, &git, &github, workspace),
        Layer::Planners | Layer::Implementers => {
            execute_issue_layer(options, &jules_path, &git, &github, workspace)
        }
    }
}

/// Execute narrator.
fn execute_narrator<G, H, W>(
    mock: bool,
    jules_path: &std::path::Path,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResults, AppError>
where
    G: crate::ports::GitPort,
    H: crate::ports::GitHubPort,
    W: WorkspaceStore,
{
    let run_options = RunOptions {
        layer: Layer::Narrators,
        roles: None,
        workstream: None,
        scheduled: false,
        prompt_preview: false,
        branch: None,
        issue: None,
        mock,
    };

    eprintln!("Executing: narrator{}", if mock { " (mock)" } else { "" });
    let _result = run::execute(jules_path, run_options, git, github, workspace)?;

    // TODO: Extract mock PR numbers and branches from result when available
    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute observers (one run per workstream+role from matrix).
fn execute_observers<G, H, W>(
    options: &WorkflowRunOptions,
    jules_path: &std::path::Path,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResults, AppError>
where
    G: crate::ports::GitPort,
    H: crate::ports::GitHubPort,
    W: WorkspaceStore,
{
    let matrix = options.matrix_json.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Matrix JSON is required for observers".to_string())
    })?;

    let include = matrix
        .get("include")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Validation("Matrix JSON must have 'include' array".to_string()))?;

    let mock_suffix = if options.mock { " (mock)" } else { "" };

    for entry in include {
        let workstream = entry
            .get("workstream")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::Validation("Observer matrix entry missing 'workstream'".to_string())
            })?;

        let role = entry.get("role").and_then(|v| v.as_str()).ok_or_else(|| {
            AppError::Validation("Observer matrix entry missing 'role'".to_string())
        })?;

        let run_options = RunOptions {
            layer: Layer::Observers,
            roles: Some(vec![role.to_string()]),
            workstream: Some(workstream.to_string()),
            scheduled: false,
            prompt_preview: false,
            branch: None,
            issue: None,
            mock: options.mock,
        };

        eprintln!(
            "Executing: observers --workstream {} --role {}{}",
            workstream, role, mock_suffix
        );
        run::execute(jules_path, run_options, git, github, workspace)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute deciders (one run per unique workstream from matrix).
fn execute_deciders<G, H, W>(
    options: &WorkflowRunOptions,
    jules_path: &std::path::Path,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResults, AppError>
where
    G: crate::ports::GitPort,
    H: crate::ports::GitHubPort,
    W: WorkspaceStore,
{
    let matrix = options.matrix_json.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Matrix JSON is required for deciders".to_string())
    })?;

    let include = matrix
        .get("include")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Validation("Matrix JSON must have 'include' array".to_string()))?;

    // Deduplicate by workstream
    let mut seen_workstreams = std::collections::HashSet::new();
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    for entry in include {
        let workstream = entry
            .get("workstream")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::Validation("Decider matrix entry missing 'workstream'".to_string())
            })?;

        if !seen_workstreams.insert(workstream.to_string()) {
            continue; // Skip duplicate workstreams
        }

        let run_options = RunOptions {
            layer: Layer::Deciders,
            roles: None, // All decider roles for this workstream
            workstream: Some(workstream.to_string()),
            scheduled: false,
            prompt_preview: false,
            branch: None,
            issue: None,
            mock: options.mock,
        };

        eprintln!("Executing: deciders --workstream {}{}", workstream, mock_suffix);
        run::execute(jules_path, run_options, git, github, workspace)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute issue-based layers (planners, implementers).
fn execute_issue_layer<G, H, W>(
    options: &WorkflowRunOptions,
    jules_path: &std::path::Path,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResults, AppError>
where
    G: crate::ports::GitPort,
    H: crate::ports::GitHubPort,
    W: WorkspaceStore,
{
    let matrix = options.matrix_json.as_ref().ok_or_else(|| {
        AppError::MissingArgument(format!(
            "Matrix JSON is required for {}",
            options.layer.dir_name()
        ))
    })?;

    let include = matrix
        .get("include")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Validation("Matrix JSON must have 'include' array".to_string()))?;

    let mock_suffix = if options.mock { " (mock)" } else { "" };

    for entry in include {
        let issue = entry.get("issue").and_then(|v| v.as_str()).ok_or_else(|| {
            AppError::Validation(format!(
                "{} matrix entry missing 'issue'",
                options.layer.dir_name()
            ))
        })?;

        let run_options = RunOptions {
            layer: options.layer,
            roles: None,
            workstream: None,
            scheduled: false,
            prompt_preview: false,
            branch: None,
            issue: Some(PathBuf::from(issue)),
            mock: options.mock,
        };

        eprintln!("Executing: {} {}{}", options.layer.dir_name(), issue, mock_suffix);
        run::execute(jules_path, run_options, git, github, workspace)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Validate matrix is provided for layers that need it.
fn validate_matrix_requirement(options: &WorkflowRunOptions) -> Result<(), AppError> {
    match options.layer {
        Layer::Narrators => Ok(()),
        Layer::Observers | Layer::Deciders | Layer::Planners | Layer::Implementers => {
            if options.matrix_json.is_none() {
                return Err(AppError::MissingArgument(format!(
                    "Matrix JSON is required for layer '{}'",
                    options.layer.dir_name()
                )));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrator_does_not_require_matrix() {
        let options = WorkflowRunOptions {
            layer: Layer::Narrators,
            matrix_json: None,
            target_branch: None,
            mock: false,
        };

        assert!(validate_matrix_requirement(&options).is_ok());
    }

    #[test]
    fn observer_requires_matrix() {
        let options = WorkflowRunOptions {
            layer: Layer::Observers,
            matrix_json: None,
            target_branch: None,
            mock: false,
        };

        assert!(validate_matrix_requirement(&options).is_err());
    }

    #[test]
    fn observer_with_matrix_is_valid() {
        let matrix = serde_json::json!({
            "include": [{"workstream": "alpha", "role": "taxonomy"}]
        });

        let options = WorkflowRunOptions {
            layer: Layer::Observers,
            matrix_json: Some(matrix),
            target_branch: None,
            mock: false,
        };

        assert!(validate_matrix_requirement(&options).is_ok());
    }
}
