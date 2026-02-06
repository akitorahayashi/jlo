//! Workflow run command implementation.
//!
//! Executes a single workstream's layer by reading scheduled.toml and running enabled roles.
//! This command provides orchestration for GitHub Actions workflows.

use chrono::Utc;
use serde::Serialize;
use std::path::Path;

use crate::app::commands::run::{self, RunOptions};
use crate::domain::{AppError, Layer};
use crate::ports::WorkspaceStore;
use crate::services::adapters::git_command::GitCommandAdapter;
use crate::services::adapters::github_command::GitHubCommandAdapter;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::services::adapters::workstream_schedule_filesystem::load_schedule;

/// Options for workflow run command.
#[derive(Debug, Clone)]
pub struct WorkflowRunOptions {
    /// Target workstream.
    pub workstream: String,
    /// Target layer.
    pub layer: Layer,
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

    // Execute layer runs for the specified workstream
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

/// Execute runs for a layer on a specific workstream.
fn execute_layer(
    options: &WorkflowRunOptions,
    workspace: &FilesystemWorkspaceStore,
) -> Result<RunResults, AppError> {
    let jules_path = workspace.jules_path();
    let git_root = jules_path.parent().unwrap_or(&jules_path).to_path_buf();
    let git = GitCommandAdapter::new(git_root);
    let github = GitHubCommandAdapter::new();

    match options.layer {
        Layer::Narrators => execute_narrator(options, &jules_path, &git, &github, workspace),
        Layer::Observers => execute_multi_role(options, &jules_path, &git, &github, workspace),
        Layer::Deciders => execute_multi_role(options, &jules_path, &git, &github, workspace),
        Layer::Planners => execute_issue_layer(options, &jules_path, &git, &github, workspace),
        Layer::Implementers => execute_issue_layer(options, &jules_path, &git, &github, workspace),
    }
}

/// Execute narrator (workstream-independent).
fn execute_narrator<G, H, W>(
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResults, AppError>
where
    G: crate::ports::GitPort,
    H: crate::ports::GitHubPort,
    W: WorkspaceStore + crate::domain::PromptAssetLoader,
{
    let run_options = RunOptions {
        layer: Layer::Narrators,
        role: None,
        workstream: None,
        prompt_preview: false,
        branch: None,
        issue: None,
        mock: options.mock,
    };

    eprintln!("Executing: narrator{}", if options.mock { " (mock)" } else { "" });
    run::execute(jules_path, run_options, git, github, workspace)?;

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute multi-role layer (observers, deciders) for a specific workstream.
fn execute_multi_role<G, H, W>(
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResults, AppError>
where
    G: crate::ports::GitPort,
    H: crate::ports::GitHubPort,
    W: WorkspaceStore + crate::domain::PromptAssetLoader,
{
    let workstream = &options.workstream;
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    // Load schedule for the workstream
    let schedule = load_schedule(jules_path, workstream)?;

    if !schedule.enabled {
        eprintln!("Workstream '{}' is disabled, skipping", workstream);
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    // Get enabled roles for the layer
    let roles = match options.layer {
        Layer::Observers => schedule.observers.enabled_roles(),
        Layer::Deciders => schedule.deciders.enabled_roles(),
        _ => {
            return Err(AppError::Validation("Invalid layer for multi-role execution".to_string()));
        }
    };

    if roles.is_empty() {
        eprintln!("No enabled {} roles for workstream '{}'", options.layer.dir_name(), workstream);
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    // Execute each role
    for role in roles {
        let run_options = RunOptions {
            layer: options.layer,
            role: Some(role.as_str().to_string()),
            workstream: Some(workstream.clone()),
            prompt_preview: false,
            branch: None,
            issue: None,
            mock: options.mock,
        };

        eprintln!(
            "Executing: {} --workstream {} --role {}{}",
            options.layer.dir_name(),
            workstream,
            role,
            mock_suffix
        );
        run::execute(jules_path, run_options, git, github, workspace)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute issue-based layers (planners, implementers) for a specific workstream.
fn execute_issue_layer<G, H, W>(
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResults, AppError>
where
    G: crate::ports::GitPort,
    H: crate::ports::GitHubPort,
    W: WorkspaceStore + crate::domain::PromptAssetLoader,
{
    let workstream = &options.workstream;
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    // Find issues for the layer in this workstream
    let issues = find_issues_for_workstream(jules_path, workstream, options.layer)?;

    if issues.is_empty() {
        eprintln!(
            "No issues found for {} in workstream '{}'",
            options.layer.dir_name(),
            workstream
        );
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    for issue_path in issues {
        let run_options = RunOptions {
            layer: options.layer,
            role: None,
            workstream: Some(workstream.clone()),
            prompt_preview: false,
            branch: None,
            issue: Some(issue_path.clone()),
            mock: options.mock,
        };

        eprintln!(
            "Executing: {} {}{}",
            options.layer.dir_name(),
            issue_path.display(),
            mock_suffix
        );
        run::execute(jules_path, run_options, git, github, workspace)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Find issues for a specific workstream and layer.
fn find_issues_for_workstream(
    jules_path: &Path,
    workstream: &str,
    layer: Layer,
) -> Result<Vec<std::path::PathBuf>, AppError> {
    let status_dir = match layer {
        Layer::Planners => "ready_for_planner",
        Layer::Implementers => "ready_for_implementer",
        _ => return Err(AppError::Validation("Invalid layer for issue discovery".to_string())),
    };

    let issues_dir =
        jules_path.join("workstreams").join(workstream).join("issues").join(status_dir);

    if !issues_dir.exists() {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();
    let entries = std::fs::read_dir(&issues_dir).map_err(AppError::Io)?;

    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md" || ext == "yml" || ext == "yaml")
                {
                    issues.push(path);
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read directory entry in {}: {}",
                    issues_dir.display(),
                    e
                );
            }
        }
    }

    issues.sort();
    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_run_options_with_workstream() {
        let options = WorkflowRunOptions {
            workstream: "generic".to_string(),
            layer: Layer::Observers,
            mock: false,
        };
        assert_eq!(options.workstream, "generic");
        assert_eq!(options.layer, Layer::Observers);
        assert!(!options.mock);
    }
}
