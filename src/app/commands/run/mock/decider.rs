use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::mock::identity::generate_mock_id;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock deciders.
pub fn execute_mock_deciders<G, H, W>(
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
        AppError::MissingArgument("Workstream is required for deciders".to_string())
    })?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Deciders, &timestamp);

    println!("Mock deciders: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    let exchange_dir = jules_path.join("workstreams").join(workstream).join("exchange");

    // Find and process pending events
    let pending_dir = exchange_dir.join("events").join("pending");
    let decided_dir = exchange_dir.join("events").join("decided");
    let issues_dir = exchange_dir.join("issues");

    // Ensure directories exist
    std::fs::create_dir_all(&decided_dir).ok();
    std::fs::create_dir_all(&issues_dir).ok();

    // Create two mock issues: one for planner, one for implementer
    let label = config.issue_labels.first().cloned().unwrap_or_else(|| "bugs".to_string());
    let label_dir = issues_dir.join(&label);
    std::fs::create_dir_all(&label_dir).ok();

    let mock_issue_template = include_str!("assets/decider_issue.yml");

    // Move any mock pending events to decided first, collecting event IDs
    let mut moved_dest_files: Vec<PathBuf> = Vec::new();
    let mut moved_src_files: Vec<PathBuf> = Vec::new();
    let mut source_event_ids: Vec<String> = Vec::new();
    if pending_dir.exists() {
        for entry in std::fs::read_dir(&pending_dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path
                .file_name()
                .map(|n| n.to_string_lossy().contains(&config.scope))
                .unwrap_or(false)
            {
                // Extract event ID from filename (format: mock-{scope}-{event_id}.yml)
                if let Some(filename) = path.file_name() {
                    let name = filename.to_string_lossy();
                    // Parse event ID from filename
                    if let Some(id_part) = name.strip_prefix(&format!("mock-{}-", config.scope))
                        && let Some(event_id) = id_part.strip_suffix(".yml")
                    {
                        source_event_ids.push(event_id.to_string());
                    }
                }

                let dest = decided_dir.join(path.file_name().unwrap());
                if std::fs::rename(&path, &dest).is_ok() {
                    moved_src_files.push(path);
                    moved_dest_files.push(dest);
                }
            }
        }
    }

    // Use first moved event as source, or generate a placeholder if none
    let source_event_id = source_event_ids
        .first()
        .cloned()
        .unwrap_or_else(|| format!("mock-event-{}", generate_mock_id()));

    // Issue 1: requires deep analysis (for planner)
    let planner_issue_id = generate_mock_id();
    let planner_issue_file = label_dir.join(format!("mock-planner-{}.yml", config.scope));
    let planner_issue_content = mock_issue_template
        .replace("mock01", &planner_issue_id)
        .replace("test-scope", &config.scope)
        .replace("event1", &source_event_id)
        .replace("requires_deep_analysis: false", "requires_deep_analysis: true\ndeep_analysis_reason: \"Mock issue requires architectural analysis for workflow validation\"")
        .replace("Mock issue for workflow validation", "Mock issue requiring deep analysis")
        .replace("medium", "high"); // Make it high priority for planner

    workspace.write_file(
        planner_issue_file
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &planner_issue_content,
    )?;

    // Issue 2: ready for implementer
    let impl_issue_id = generate_mock_id();
    let impl_issue_file = label_dir.join(format!("mock-impl-{}.yml", config.scope));
    let impl_issue_content = mock_issue_template
        .replace("mock01", &impl_issue_id)
        .replace("test-scope", &config.scope)
        .replace("event1", &source_event_id)
        .replace("Mock issue for workflow validation", "Mock issue ready for implementation");

    workspace.write_file(
        impl_issue_file.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &impl_issue_content,
    )?;

    // Update moved event files to set issue_id (decided events must have issue_id)
    for dest_file in &moved_dest_files {
        if let Ok(content) = std::fs::read_to_string(dest_file) {
            // Update issue_id in the event file
            let updated_content =
                content.replace("issue_id: \"\"", &format!("issue_id: \"{}\"", impl_issue_id));
            std::fs::write(dest_file, updated_content).ok();
        }
    }

    // Commit and push (include moved event files)
    let mut files: Vec<&Path> = vec![planner_issue_file.as_path(), impl_issue_file.as_path()];
    for f in &moved_dest_files {
        files.push(f.as_path());
    }
    for f in &moved_src_files {
        files.push(f.as_path());
    }
    git.commit_files(&format!("[mock-{}] decider: mock issues", config.scope), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Decider triage", config.scope),
        &format!("Mock decider run for workflow validation.\n\nScope: `{}`\nWorkstream: `{}`\n\nCreated issues:\n- `{}` (requires analysis)\n- `{}` (ready for impl)", 
            config.scope, workstream, planner_issue_id, impl_issue_id),
    )?;

    println!("Mock deciders: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_scope: config.scope.clone(),
    })
}
