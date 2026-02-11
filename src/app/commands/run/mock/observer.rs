use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::adapters::schedule_filesystem::load_schedule;
use crate::app::commands::run::RunOptions;
use crate::app::commands::run::mock::identity::generate_mock_id;
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

// Template placeholder constants (must match src/assets/mock/observer_event.yml)
const TMPL_ID: &str = "mock01";
const TMPL_DATE: &str = "2026-02-05";
const TMPL_TAG: &str = "test-tag";

// Comment template placeholder constants (must match src/assets/mock/observer_comment.yml)
const COMMENT_TMPL_AUTHOR: &str = "mock-author";
const COMMENT_TMPL_TAG: &str = "test-tag";

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
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Observers, &timestamp)?;

    println!("Mock observers: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create mock events
    let events_dir = jules::events_pending_dir(jules_path);

    let mock_event_template = super::MOCK_ASSETS
        .get_file("observer_event.yml")
        .ok_or_else(|| {
            AppError::InternalError("Mock asset missing: observer_event.yml".to_string())
        })?
        .contents_utf8()
        .ok_or_else(|| {
            AppError::InternalError("Invalid UTF-8 in observer_event.yml".to_string())
        })?;

    let observer_role = options.role.as_deref().unwrap_or("mock");
    if !validate_safe_path_component(observer_role) {
        return Err(AppError::Validation(format!(
            "Invalid role name '{}': must be alphanumeric with hyphens or underscores only",
            observer_role
        )));
    }

    // Create mock event 1 (for planner routing)
    let event_id_1 = generate_mock_id();
    let event_file_1 = events_dir.join(format!("mock-{}-{}.yml", config.mock_tag, event_id_1));
    let event_content_1 = mock_event_template
        .replace(TMPL_ID, &event_id_1)
        .replace(TMPL_DATE, &Utc::now().format("%Y-%m-%d").to_string())
        .replace(TMPL_TAG, &config.mock_tag);

    // Create mock event 2 (for implementer routing)
    let event_id_2 = generate_mock_id();
    let event_file_2 = events_dir.join(format!("mock-{}-{}.yml", config.mock_tag, event_id_2));
    let event_content_2 = mock_event_template
        .replace(TMPL_ID, &event_id_2)
        .replace(TMPL_DATE, &Utc::now().format("%Y-%m-%d").to_string())
        .replace(TMPL_TAG, &config.mock_tag)
        .replace("workflow validation", "workflow implementation check");

    // Ensure directory exists
    workspace.create_dir_all(
        events_dir.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
    )?;

    workspace.write_file(
        event_file_1.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &event_content_1,
    )?;

    workspace.write_file(
        event_file_2.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &event_content_2,
    )?;

    // Bridge: generate comment artifacts for each scheduled innovator persona.
    // This enables the innovator refinement stage to consume observer feedback
    // without manual edits. The comment filename is deterministic to prevent
    // uncontrolled duplicates on re-runs.
    let mut comment_files: Vec<PathBuf> = Vec::new();
    let innovator_personas = resolve_innovator_personas(workspace);

    if !innovator_personas.is_empty() {
        let mock_comment_template = super::MOCK_ASSETS
            .get_file("observer_comment.yml")
            .ok_or_else(|| {
                AppError::InternalError("Mock asset missing: observer_comment.yml".to_string())
            })?
            .contents_utf8()
            .ok_or_else(|| {
                AppError::InternalError("Invalid UTF-8 in observer_comment.yml".to_string())
            })?;

        for persona in &innovator_personas {
            let comments_dir = jules::innovator_comments_dir(jules_path, persona);

            let comments_dir_str = comments_dir
                .to_str()
                .ok_or_else(|| AppError::Validation("Invalid comments path".to_string()))?;
            workspace.create_dir_all(comments_dir_str)?;

            // Deterministic filename: observer-{observer_role}-{mock_tag}.yml
            // Uses mock_tag (not event_id) to ensure idempotency per run.
            let comment_file =
                comments_dir.join(format!("observer-{}-{}.yml", observer_role, config.mock_tag));
            let comment_content = mock_comment_template
                .replace(COMMENT_TMPL_AUTHOR, observer_role)
                .replace(COMMENT_TMPL_TAG, &config.mock_tag);

            workspace.write_file(
                comment_file
                    .to_str()
                    .ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
                &comment_content,
            )?;
            comment_files.push(comment_file);
        }

        println!(
            "Mock observers: created {} comment(s) for innovator personas",
            comment_files.len()
        );
    }

    // Commit and push
    let mut all_files: Vec<&Path> = vec![event_file_1.as_path(), event_file_2.as_path()];
    for cf in &comment_files {
        all_files.push(cf.as_path());
    }
    git.commit_files(&format!("[{}] observer: mock event", config.mock_tag), &all_files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[{}] Observer findings", config.mock_tag),
        &format!("Mock observer run for workflow validation.\n\nMock tag: `{}`", config.mock_tag),
    )?;

    println!("Mock observers: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}

/// Resolve enabled innovator persona names.
///
/// Returns an empty vec if the schedule is missing, innovators are not
/// configured, or an error occurs (silent fallback is acceptable here
/// because the comment bridge is supplementary output, not a primary
/// deliverable of the observer mock).
fn resolve_innovator_personas<W: WorkspaceStore>(workspace: &W) -> Vec<String> {
    let schedule = match load_schedule(workspace) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let Some(ref innovators) = schedule.innovators else {
        return Vec::new();
    };
    innovators.enabled_roles().iter().map(|r| r.as_str().to_string()).collect()
}
