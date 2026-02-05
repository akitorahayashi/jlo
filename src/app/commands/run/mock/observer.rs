use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::mock::identity::generate_mock_id;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock observers.
pub fn execute_mock_observers<G, H, W>(
    jules_path: &Path,
    options: &RunOptions,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore,
{
    let workstream = options.workstream.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Workstream is required for observers".to_string())
    })?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Observers, &timestamp);

    println!("Mock observers: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create mock events
    let events_dir = jules_path
        .join("workstreams")
        .join(workstream)
        .join("exchange")
        .join("events")
        .join("pending");

    let mock_event_template = include_str!("assets/observer_event.yml");

    // Create mock event 1 (for planner routing)
    let event_id_1 = generate_mock_id();
    let event_file_1 = events_dir.join(format!("mock-{}-{}.yml", config.scope, event_id_1));
    let event_content_1 = mock_event_template
        .replace("mock01", &event_id_1)
        .replace("2026-02-05", &Utc::now().format("%Y-%m-%d").to_string())
        .replace("test-scope", &config.scope);

    // Create mock event 2 (for implementer routing)
    let event_id_2 = generate_mock_id();
    let event_file_2 = events_dir.join(format!("mock-{}-{}.yml", config.scope, event_id_2));
    let event_content_2 = mock_event_template
        .replace("mock01", &event_id_2)
        .replace("2026-02-05", &Utc::now().format("%Y-%m-%d").to_string())
        .replace("test-scope", &config.scope)
        .replace("workflow validation", "workflow implementation check");

    // Ensure directory exists
    std::fs::create_dir_all(&events_dir).map_err(AppError::Io)?;

    workspace.write_file(
        event_file_1.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &event_content_1,
    )?;

    workspace.write_file(
        event_file_2.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &event_content_2,
    )?;

    // Commit and push
    let files: Vec<&Path> = vec![event_file_1.as_path(), event_file_2.as_path()];
    git.commit_files(&format!("[mock-{}] observer: mock event", config.scope), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Observer findings", config.scope),
        &format!(
            "Mock observer run for workflow validation.\n\nScope: `{}`\nWorkstream: `{}`",
            config.scope, workstream
        ),
    )?;

    println!("Mock observers: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_scope: config.scope.clone(),
    })
}
