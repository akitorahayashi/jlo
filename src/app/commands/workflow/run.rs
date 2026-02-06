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
use crate::services::adapters::issue_filesystem::read_issue_header;
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
    if layer != Layer::Planners && layer != Layer::Implementers {
        return Err(AppError::Validation("Invalid layer for issue discovery".to_string()));
    }

    let issues_root =
        jules_path.join("workstreams").join(workstream).join("exchange").join("issues");

    if !issues_root.exists() {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();
    let routing_labels = resolve_routing_labels(&issues_root)?;

    for label in routing_labels {
        let label_dir = issues_root.join(&label);
        if !label_dir.exists() {
            continue;
        }

        let entries = std::fs::read_dir(&label_dir).map_err(AppError::Io)?;
        for entry in entries {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    let is_issue_file =
                        path.extension().is_some_and(|ext| ext == "yml" || ext == "yaml");
                    if !is_issue_file {
                        continue;
                    }

                    let header = read_issue_header(&path)?;
                    let requires_deep_analysis = header.requires_deep_analysis;
                    let belongs_to_layer = match layer {
                        Layer::Planners => requires_deep_analysis,
                        Layer::Implementers => !requires_deep_analysis,
                        _ => false,
                    };
                    if belongs_to_layer {
                        issues.push(path);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to read directory entry in {}: {}",
                        label_dir.display(),
                        e
                    );
                }
            }
        }
    }

    issues.sort();
    Ok(issues)
}

fn resolve_routing_labels(issues_root: &Path) -> Result<Vec<String>, AppError> {
    if let Ok(labels_csv) = std::env::var("ROUTING_LABELS") {
        let labels: Vec<String> = labels_csv
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect();

        if labels.is_empty() {
            return Err(AppError::Validation(
                "ROUTING_LABELS is set but does not contain any labels".to_string(),
            ));
        }
        return Ok(labels);
    }

    eprintln!("ROUTING_LABELS is not set; discovering labels from {}", issues_root.display());
    let mut discovered = Vec::new();
    let entries = std::fs::read_dir(issues_root).map_err(AppError::Io)?;
    for entry in entries {
        let entry = entry.map_err(AppError::Io)?;
        if entry.path().is_dir() {
            discovered.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    discovered.sort();
    if discovered.is_empty() {
        return Err(AppError::Validation(format!(
            "No issue label directories found under {}",
            issues_root.display()
        )));
    }

    Ok(discovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

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

    fn setup_workspace(root: &Path) {
        fs::create_dir_all(root.join(".jules")).unwrap();
        fs::write(root.join(".jules/version"), env!("CARGO_PKG_VERSION")).unwrap();
    }

    fn write_issue(root: &Path, label: &str, name: &str, requires_deep_analysis: bool) {
        let issue_dir = root.join(".jules/workstreams/alpha/exchange/issues").join(label);
        fs::create_dir_all(&issue_dir).unwrap();
        let content = format!(
            "id: test01\nrequires_deep_analysis: {}\nsource_events:\n  - event1\n",
            requires_deep_analysis
        );
        fs::write(issue_dir.join(format!("{}.yml", name)), content).unwrap();
    }

    #[test]
    #[serial]
    fn planner_issue_discovery_filters_by_requires_deep_analysis() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        write_issue(root, "bugs", "requires-planning", true);
        write_issue(root, "bugs", "ready-to-implement", false);
        write_issue(root, "docs", "ignored-by-routing", true);

        let jules_path = root.join(".jules");

        unsafe {
            std::env::set_var("ROUTING_LABELS", "bugs");
        }
        let issues = find_issues_for_workstream(&jules_path, "alpha", Layer::Planners).unwrap();
        unsafe {
            std::env::remove_var("ROUTING_LABELS");
        }

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("requires-planning.yml"));
    }

    #[test]
    #[serial]
    fn implementer_issue_discovery_uses_non_deep_issues() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        write_issue(root, "bugs", "requires-planning", true);
        write_issue(root, "bugs", "ready-to-implement", false);

        let jules_path = root.join(".jules");

        unsafe {
            std::env::set_var("ROUTING_LABELS", "bugs");
        }
        let issues = find_issues_for_workstream(&jules_path, "alpha", Layer::Implementers).unwrap();
        unsafe {
            std::env::remove_var("ROUTING_LABELS");
        }

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("ready-to-implement.yml"));
    }
}
