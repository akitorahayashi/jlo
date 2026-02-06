//! Workflow run command implementation.
//!
//! Executes a single workstream's layer by reading scheduled.toml and running enabled roles.
//! This command provides orchestration for GitHub Actions workflows.

use chrono::Utc;
use serde::Serialize;
use std::path::Path;

use crate::app::commands::run::{self, RunOptions};
use crate::domain::{AppError, Layer};
use crate::ports::{GitPort, GitHubPort, WorkspaceStore};

use crate::adapters::workstream_schedule_filesystem::load_schedule;

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
pub fn execute<G, H>(
    store: &impl WorkspaceStore,
    options: WorkflowRunOptions,
    git: &G,
    github: &H,
) -> Result<WorkflowRunOutput, AppError>
where
    G: GitPort,
    H: GitHubPort,
{
    if !store.exists() {
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
    let run_results = execute_layer(store, &options, git, github)?;

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
fn execute_layer<G, H>(
    store: &impl WorkspaceStore,
    options: &WorkflowRunOptions,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    G: GitPort,
    H: GitHubPort,
{
    let jules_path = store.jules_path();

    match options.layer {
        Layer::Narrators => execute_narrator(store, options, &jules_path, git, github),
        Layer::Observers => execute_multi_role(store, options, &jules_path, git, github),
        Layer::Deciders => execute_multi_role(store, options, &jules_path, git, github),
        Layer::Planners => execute_issue_layer(store, options, &jules_path, git, github),
        Layer::Implementers => execute_issue_layer(store, options, &jules_path, git, github),
    }
}

/// Execute narrator (workstream-independent).
fn execute_narrator<G, H>(
    store: &impl WorkspaceStore,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    G: GitPort,
    H: GitHubPort,
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
    run::execute(jules_path, run_options, git, github, store)?;

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute multi-role layer (observers, deciders) for a specific workstream.
fn execute_multi_role<G, H>(
    store: &impl WorkspaceStore,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    G: GitPort,
    H: GitHubPort,
{
    let workstream = &options.workstream;
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    // Load schedule for the workstream
    let schedule = load_schedule(store, workstream)?;

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
        run::execute(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute issue-based layers (planners, implementers) for a specific workstream.
fn execute_issue_layer<G, H>(
    store: &impl WorkspaceStore,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    G: GitPort,
    H: GitHubPort,
{
    let workstream = &options.workstream;
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    // Find issues for the layer in this workstream
    let issues = find_issues_for_workstream(store, workstream, options.layer)?;

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
        run::execute(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Find issues for a specific workstream and layer.
fn find_issues_for_workstream(
    store: &impl WorkspaceStore,
    workstream: &str,
    layer: Layer,
) -> Result<Vec<std::path::PathBuf>, AppError> {
    if layer != Layer::Planners && layer != Layer::Implementers {
        return Err(AppError::Validation("Invalid layer for issue discovery".to_string()));
    }

    let jules_path = store.jules_path();
    let issues_root =
        jules_path.join("workstreams").join(workstream).join("exchange").join("issues");

    if !store.file_exists(issues_root.to_str().unwrap()) {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();
    let routing_labels = resolve_routing_labels(store, &issues_root)?;

    for label in routing_labels {
        let label_dir = issues_root.join(&label);
        if !store.file_exists(label_dir.to_str().unwrap()) {
            continue;
        }

        let entries = store.list_dir(label_dir.to_str().unwrap())?;
        for path in entries {
            let is_issue_file = path.extension().is_some_and(|ext| ext == "yml" || ext == "yaml");
            if !is_issue_file {
                continue;
            }

            let requires_deep_analysis = read_requires_deep_analysis(store, &path)?;
            let belongs_to_layer = match layer {
                Layer::Planners => requires_deep_analysis,
                Layer::Implementers => !requires_deep_analysis,
                _ => false,
            };
            if belongs_to_layer {
                issues.push(path);
            }
        }
    }

    issues.sort();
    Ok(issues)
}

fn resolve_routing_labels(
    store: &impl WorkspaceStore,
    issues_root: &Path,
) -> Result<Vec<String>, AppError> {
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

        for label in &labels {
            if label.contains("..") || label.contains('/') || label.contains('\\') {
                return Err(AppError::Validation(format!(
                    "Invalid routing label '{}': must not contain path separators or '..'",
                    label
                )));
            }
        }

        return Ok(labels);
    }

    eprintln!("ROUTING_LABELS is not set; discovering labels from {}", issues_root.display());
    let mut discovered = Vec::new();
    let entries = store.list_dir(issues_root.to_str().unwrap())?;
    for path in entries {
        if store.is_dir(path.to_str().unwrap()) {
            discovered.push(path.file_name().unwrap().to_string_lossy().to_string());
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

fn read_requires_deep_analysis(store: &impl WorkspaceStore, path: &Path) -> Result<bool, AppError> {
    let content = store.read_file(path.to_str().unwrap())?;
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|error| {
        AppError::ParseError { what: path.display().to_string(), details: error.to_string() }
    })?;

    match &parsed["requires_deep_analysis"] {
        serde_yaml::Value::Bool(value) => Ok(*value),
        _ => Err(AppError::Validation(format!(
            "Missing or invalid requires_deep_analysis in {}",
            path.display()
        ))),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::WorkspaceStore;
    use crate::adapters::memory_workspace_store::MemoryWorkspaceStore;
    use serial_test::serial;

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

    fn setup_workspace(store: &MemoryWorkspaceStore) {
        store.write_version(env!("CARGO_PKG_VERSION")).unwrap();
    }

    fn write_issue(
        store: &MemoryWorkspaceStore,
        label: &str,
        name: &str,
        requires_deep_analysis: bool,
    ) {
        let issue_dir = format!(".jules/workstreams/alpha/exchange/issues/{}", label);
        let content = format!(
            "id: test01\nrequires_deep_analysis: {}\nsource_events:\n  - event1\n",
            requires_deep_analysis
        );
        let path = format!("{}/{}.yml", issue_dir, name);
        store.write_file(&path, &content).unwrap();
    }

    #[test]
    #[serial]
    fn planner_issue_discovery_filters_by_requires_deep_analysis() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        write_issue(&store, "bugs", "requires-planning", true);
        write_issue(&store, "bugs", "ready-to-implement", false);
        write_issue(&store, "docs", "ignored-by-routing", true);

        unsafe {
            std::env::set_var("ROUTING_LABELS", "bugs");
        }
        let issues = find_issues_for_workstream(&store, "alpha", Layer::Planners).unwrap();
        unsafe {
            std::env::remove_var("ROUTING_LABELS");
        }

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("requires-planning.yml"));
    }

    #[test]
    #[serial]
    fn implementer_issue_discovery_uses_non_deep_issues() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        write_issue(&store, "bugs", "requires-planning", true);
        write_issue(&store, "bugs", "ready-to-implement", false);

        unsafe {
            std::env::set_var("ROUTING_LABELS", "bugs");
        }
        let issues = find_issues_for_workstream(&store, "alpha", Layer::Implementers).unwrap();
        unsafe {
            std::env::remove_var("ROUTING_LABELS");
        }

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("ready-to-implement.yml"));
    }
}
