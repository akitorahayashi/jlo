use crate::adapters::schedule_filesystem::load_schedule;
use crate::app::commands::run::{self, RunOptions};
use crate::domain::{AppError, Layer};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};
use std::path::Path;

use super::issue_routing::find_issues;
use super::options::{RunResults, WorkflowRunOptions};

/// Execute runs for a layer on a specific workstream.
pub(crate) fn execute_layer<G, H>(
    store: &(impl WorkspaceStore + Clone + Send + Sync + 'static),
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
        Layer::Innovators => execute_multi_role(store, options, &jules_path, git, github),
    }
}

/// Execute narrator (workstream-independent).
fn execute_narrator<G, H>(
    store: &(impl WorkspaceStore + Clone + Send + Sync + 'static),
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
        prompt_preview: false,
        branch: None,
        issue: None,
        mock: options.mock,
        phase: None,
    };

    eprintln!("Executing: narrator{}", if options.mock { " (mock)" } else { "" });
    run::execute(jules_path, run_options, git, github, store)?;

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute multi-role layer (observers, deciders) for a specific workstream.
fn execute_multi_role<G, H>(
    store: &(impl WorkspaceStore + Clone + Send + Sync + 'static),
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    G: GitPort,
    H: GitHubPort,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    // Load root schedule
    let schedule = load_schedule(store)?;

    if !schedule.enabled {
        eprintln!("Schedule is disabled, skipping");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    // Get enabled roles for the layer
    let roles = match options.layer {
        Layer::Observers => schedule.observers.enabled_roles(),
        Layer::Deciders => schedule.deciders.enabled_roles(),
        Layer::Innovators => {
            schedule.innovators.as_ref().map(|l| l.enabled_roles()).unwrap_or_default()
        }
        _ => {
            return Err(AppError::Validation("Invalid layer for multi-role execution".to_string()));
        }
    };

    if roles.is_empty() {
        eprintln!("No enabled {} roles", options.layer.dir_name());
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    // Execute each role
    for role in roles {
        let run_options = RunOptions {
            layer: options.layer,
            role: Some(role.as_str().to_string()),
            prompt_preview: false,
            branch: None,
            issue: None,
            mock: options.mock,
            phase: options.phase.clone(),
        };

        eprintln!("Executing: {} --role {}{}", options.layer.dir_name(), role, mock_suffix);
        run::execute(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute issue-based layers (planners, implementers) for a specific workstream.
fn execute_issue_layer<G, H>(
    store: &(impl WorkspaceStore + Clone + Send + Sync + 'static),
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    G: GitPort,
    H: GitHubPort,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    // Find issues for the layer
    let issues = find_issues(store, options.layer, options.routing_labels.as_deref())?;

    if issues.is_empty() {
        eprintln!("No issues found for {}", options.layer.dir_name(),);
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    for issue_path in issues {
        let run_options = RunOptions {
            layer: options.layer,
            role: None,
            prompt_preview: false,
            branch: None,
            issue: Some(issue_path.clone()),
            mock: options.mock,
            phase: None,
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
