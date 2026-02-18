use std::path::Path;

use chrono::Utc;

use super::super::mock::mock_execution::{MOCK_ASSETS, generate_mock_id};
use crate::app::commands::run::RunRuntimeOptions;
use crate::app::commands::run::input::{detect_repository_source, load_mock_config};
use crate::domain::layers::execute::starting_branch::resolve_starting_branch;
use crate::domain::layers::prompt_assemble::{
    AssembledPrompt, PromptAssetLoader, PromptContext, assemble_prompt,
};
use crate::domain::{
    AppError, ControlPlaneConfig, Layer, MockConfig, MockOutput, RoleId, RunOptions,
};
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};

use super::super::role_session::{dispatch_session, print_role_preview, validate_role_exists};
use super::super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct ObserversLayer;

impl<W> LayerStrategy<W> for ObserversLayer
where
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        target: &RunOptions,
        runtime: &RunRuntimeOptions,
        config: &ControlPlaneConfig,
        git: &dyn Git,
        github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if runtime.mock {
            let role_str = target.role.clone().ok_or_else(|| {
                AppError::MissingArgument("Role is required for observers in mock mode".to_string())
            })?;
            let role = RoleId::new(&role_str)?;
            let mock_config = load_mock_config(jules_path, repository)?;
            let output = execute_mock(jules_path, &role, &mock_config, git, github, repository)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::super::mock::mock_execution::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::super::mock::mock_execution::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec![role.to_string()],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            runtime.prompt_preview,
            runtime.branch.as_deref(),
            target.role.as_deref(),
            config,
            git,
            repository,
            client_factory,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    role: Option<&str>,
    config: &ControlPlaneConfig,
    git: &G,
    repository: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
    G: Git + ?Sized,
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    let role = role
        .ok_or_else(|| AppError::MissingArgument("Role is required for observers".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Observers, role_id.as_str(), repository)?;

    let starting_branch = resolve_starting_branch(Layer::Observers, config, branch);

    if prompt_preview {
        print_role_preview(jules_path, Layer::Observers, &role_id, &starting_branch, repository);
        let assembled = assemble_observer_prompt(jules_path, role_id.as_str(), repository)?;
        println!("  Assembled prompt: {} chars", assembled.len());
        println!("\nWould execute 1 session");
        return Ok(RunResult {
            roles: vec![role.to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let source = detect_repository_source(git)?;
    let assembled = assemble_observer_prompt(jules_path, role_id.as_str(), repository)?;
    let client = client_factory.create()?;

    let session_id = dispatch_session(
        Layer::Observers,
        &role_id,
        assembled,
        &source,
        starting_branch,
        client.as_ref(),
    )?;

    Ok(RunResult {
        roles: vec![role.to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
        cleanup_requirement: None,
    })
}

fn assemble_observer_prompt<
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
>(
    jules_path: &Path,
    role: &str,
    repository: &W,
) -> Result<String, AppError> {
    let context = PromptContext::new().with_var("role", role);

    assemble_prompt(jules_path, Layer::Observers, &context, repository)
        .map(|p: AssembledPrompt| p.content)
        .map_err(|e| AppError::InternalError(e.to_string()))
}

// Template placeholder constants (must match src/assets/mock/observer_event.yml)
const TMPL_ID: &str = "mock01";
const TMPL_DATE: &str = "2026-02-05";
const TMPL_TAG: &str = "test-tag";

fn execute_mock<G, H, W>(
    jules_path: &Path,
    _observer_role: &RoleId,
    config: &MockConfig,
    git: &G,
    github: &H,
    repository: &W,
) -> Result<MockOutput, AppError>
where
    G: Git + ?Sized,
    H: GitHub + ?Sized,
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Observers, &timestamp)?;

    println!("Mock observers: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_worker_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create mock events
    let events_dir = crate::domain::exchange::events::paths::events_pending_dir(jules_path);

    let mock_event_template = MOCK_ASSETS
        .get_file("observer_event.yml")
        .ok_or_else(|| {
            AppError::InternalError("Mock asset missing: observer_event.yml".to_string())
        })?
        .contents_utf8()
        .ok_or_else(|| {
            AppError::InternalError("Invalid UTF-8 in observer_event.yml".to_string())
        })?;

    // Create mock event 1 (for planner routing)
    let event_id_1 = generate_mock_id();
    let event_file_1 = events_dir.join(format!("{}-{}.yml", config.mock_tag, event_id_1));
    let event_content_1 = mock_event_template
        .replace(TMPL_ID, &event_id_1)
        .replace(TMPL_DATE, &Utc::now().format("%Y-%m-%d").to_string())
        .replace(TMPL_TAG, &config.mock_tag);

    // Create mock event 2 (for implementer routing)
    let event_id_2 = generate_mock_id();
    let event_file_2 = events_dir.join(format!("{}-{}.yml", config.mock_tag, event_id_2));
    let event_content_2 = mock_event_template
        .replace(TMPL_ID, &event_id_2)
        .replace(TMPL_DATE, &Utc::now().format("%Y-%m-%d").to_string())
        .replace(TMPL_TAG, &config.mock_tag)
        .replace("workflow validation", "workflow implementation check");

    // Ensure directory exists
    repository.create_dir_all(
        events_dir.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
    )?;

    repository.write_file(
        event_file_1.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &event_content_1,
    )?;

    repository.write_file(
        event_file_2.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &event_content_2,
    )?;

    // Commit and push
    let all_files: Vec<&Path> = vec![event_file_1.as_path(), event_file_2.as_path()];
    git.commit_files(&format!("[{}] observer: mock event", config.mock_tag), &all_files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_worker_branch,
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
