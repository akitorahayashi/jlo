use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::domain::configuration::loader::{detect_repository_source, load_schedule};
use crate::domain::configuration::mock_loader::load_mock_config;
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::layers::mock_utils::{MOCK_ASSETS, generate_mock_id};
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::workspace::paths::jules;
use crate::domain::{
    AppError, IoErrorKind, Layer, MockConfig, MockOutput, RoleId, RunConfig, RunOptions,
};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

use super::multi_role::{dispatch_session, print_role_preview, validate_role_exists};
use super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct ObserversLayer;

impl<W> LayerStrategy<W> for ObserversLayer
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn GitPort,
        github: &dyn GitHubPort,
        workspace: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if options.mock {
            let role = options.role.clone().ok_or_else(|| {
                AppError::MissingArgument("Role is required for observers in mock mode".to_string())
            })?;
            let mock_config = load_mock_config(jules_path, options, workspace)?;
            let output = execute_mock(jules_path, options, &mock_config, git, github, workspace)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::mock_utils::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::mock_utils::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec![role],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            options.prompt_preview,
            options.branch.as_deref(),
            options.role.as_deref(),
            config,
            workspace,
            client_factory,
        )
    }
}

fn execute_real<W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    role: Option<&str>,
    config: &RunConfig,
    workspace: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let role = role
        .ok_or_else(|| AppError::MissingArgument("Role is required for observers".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Observers, role_id.as_str(), workspace)?;

    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_branch.clone());

    let bridge_task = resolve_observer_bridge_task(jules_path, workspace)?;

    if prompt_preview {
        print_role_preview(jules_path, Layer::Observers, &role_id, &starting_branch, workspace);
        let assembled =
            assemble_observer_prompt(jules_path, role_id.as_str(), &bridge_task, workspace)?;
        println!("  Assembled prompt: {} chars", assembled.len());
        println!("\nWould execute 1 session");
        return Ok(RunResult {
            roles: vec![role.to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let source = detect_repository_source()?;
    let assembled =
        assemble_observer_prompt(jules_path, role_id.as_str(), &bridge_task, workspace)?;
    let client = client_factory.create()?;

    let session_id = dispatch_session(
        Layer::Observers,
        &role_id,
        assembled,
        &source,
        &starting_branch,
        client.as_ref(),
    )?;

    Ok(RunResult {
        roles: vec![role.to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
        cleanup_requirement: None,
    })
}

fn assemble_observer_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    role: &str,
    bridge_task: &str,
    workspace: &W,
) -> Result<String, AppError> {
    let context = PromptContext::new().with_var("role", role).with_var("bridge_task", bridge_task);

    assemble_prompt(jules_path, Layer::Observers, &context, workspace)
        .map(|p: AssembledPrompt| p.content)
        .map_err(|e| AppError::InternalError(e.to_string()))
}

fn resolve_observer_bridge_task<W: WorkspaceStore>(
    jules_path: &Path,
    workspace: &W,
) -> Result<String, AppError> {
    let innovators = jules::innovators_dir(jules_path);
    let innovators_str = innovators.to_string_lossy();

    let entries = match workspace.list_dir(&innovators_str) {
        Ok(entries) => entries,
        Err(AppError::Io { kind: IoErrorKind::NotFound, .. }) => return Ok(String::new()),
        Err(err) => return Err(err),
    };

    let has_ideas = entries.iter().any(|entry| {
        workspace.is_dir(&entry.to_string_lossy())
            && workspace.file_exists(&entry.join("idea.yml").to_string_lossy())
    });

    if !has_ideas {
        return Ok(String::new());
    }

    let bridge_path = jules::tasks_dir(jules_path, Layer::Observers).join("bridge_comments.yml");
    workspace.read_file(&bridge_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "Innovator ideas exist, but observer bridge task file is missing: expected {}",
            bridge_path.display()
        ))
    })
}

// Template placeholder constants (must match src/assets/mock/observer_event.yml)
const TMPL_ID: &str = "mock01";
const TMPL_DATE: &str = "2026-02-05";
const TMPL_TAG: &str = "test-tag";

// Comment template placeholder constants (must match src/assets/mock/observer_comment.yml)
const COMMENT_TMPL_AUTHOR: &str = "mock-author";
const COMMENT_TMPL_TAG: &str = "test-tag";

fn execute_mock<G, H, W>(
    jules_path: &Path,
    options: &RunOptions,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort + ?Sized,
    H: GitHubPort + ?Sized,
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

    let mock_event_template = MOCK_ASSETS
        .get_file("observer_event.yml")
        .ok_or_else(|| {
            AppError::InternalError("Mock asset missing: observer_event.yml".to_string())
        })?
        .contents_utf8()
        .ok_or_else(|| {
            AppError::InternalError("Invalid UTF-8 in observer_event.yml".to_string())
        })?;

    let observer_role = options.role.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Role is required for observers in mock mode".to_string())
    })?;
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
    let mut comment_files: Vec<PathBuf> = Vec::new();
    let innovator_personas = resolve_innovator_personas(workspace)?;

    if !innovator_personas.is_empty() {
        let mock_comment_template = MOCK_ASSETS
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

fn resolve_innovator_personas<W: WorkspaceStore>(workspace: &W) -> Result<Vec<String>, AppError> {
    let schedule = match load_schedule(workspace) {
        Ok(schedule) => schedule,
        Err(AppError::ScheduleConfigMissing(_)) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let Some(ref innovators) = schedule.innovators else {
        return Ok(Vec::new());
    };
    Ok(innovators.enabled_roles().iter().map(|r| r.as_str().to_string()).collect())
}
