//! Run command implementation for executing Jules agents.

mod config;
mod config_dto;
pub mod mock;

pub use config::load_config;
pub use config::parse_config_content;
pub mod narrator;
pub(crate) mod narrator_logic;
mod prompt;
pub mod single_role;

use std::path::Path;
use std::path::PathBuf;

use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

use self::config::{detect_repository_source, load_config as _load_config};
use self::prompt::assemble_prompt;
use crate::ports::{AutomationMode, JulesClient, SessionRequest};

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific role to run (required for observers/deciders).
    pub role: Option<String>,
    /// Workstream (required for observers/deciders).
    pub workstream: Option<String>,
    /// Show assembled prompts without executing.
    pub prompt_preview: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Local issue file path (required for issue-driven layers: planners, implementers).
    pub issue: Option<PathBuf>,
    /// Run in mock mode (no Jules API, real git/GitHub operations).
    pub mock: bool,
}

/// Result of a run execution.
#[derive(Debug)]
pub struct RunResult {
    /// Role that was processed.
    pub roles: Vec<String>,
    /// Whether this was a prompt preview.
    pub prompt_preview: bool,
    /// Session IDs from Jules (empty if prompt_preview or mock).
    pub sessions: Vec<String>,
}

/// Execute the run command.
pub fn execute<G, H, W, C>(
    jules_path: &Path,
    options: RunOptions,
    git: &G,
    github: &H,
    workspace: &W,
    client: Option<&C>,
) -> Result<RunResult, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore + crate::domain::PromptAssetLoader,
    C: JulesClient,
{
    // Handle mock mode
    if options.mock {
        return mock::execute(jules_path, &options, git, github, workspace);
    }

    // Narrator is single-role but not issue-driven
    if options.layer == Layer::Narrators {
        return narrator::execute(
            jules_path,
            options.prompt_preview,
            options.branch.as_deref(),
            git,
            workspace,
            client,
        );
    }

    // Check if we are in CI environment (for issue-driven and single-role layers)
    let is_ci = std::env::var("GITHUB_ACTIONS").is_ok();

    // Issue-driven layers (Planners, Implementers) require an issue path
    if options.layer.is_issue_driven() {
        let issue_path = options.issue.as_deref().ok_or_else(|| {
            AppError::MissingArgument(
                "Issue path is required for issue-driven layers but was not provided.".to_string(),
            )
        })?;
        return single_role::execute(
            jules_path,
            options.layer,
            issue_path,
            options.prompt_preview,
            options.branch.as_deref(),
            is_ci,
            github,
            workspace,
            client,
        );
    }

    // Observers/Deciders: single role execution
    execute_single_role(jules_path, &options, workspace, client)
}

/// Execute a single observer or decider role.
fn execute_single_role<W: WorkspaceStore + crate::domain::PromptAssetLoader, C: JulesClient>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
    client: Option<&C>,
) -> Result<RunResult, AppError> {
    let role = options.role.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Role is required for observers/deciders".to_string())
    })?;
    let workstream = options.workstream.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Workstream is required for observers/deciders".to_string())
    })?;

    // Validate role exists
    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, options.layer, role_id.as_str())?;

    // Load config
    let config = _load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    if options.prompt_preview {
        execute_prompt_preview(
            jules_path,
            options.layer,
            &role_id,
            workstream,
            &starting_branch,
            workspace,
        )?;
        return Ok(RunResult { roles: vec![role.clone()], prompt_preview: true, sessions: vec![] });
    }

    // Determine repository source from git
    let source = detect_repository_source()?;

    let client = client.ok_or_else(|| AppError::InternalError("Client required".into()))?;

    // Execute with Jules API
    let session_id = execute_session(
        jules_path,
        options.layer,
        &role_id,
        workstream,
        &starting_branch,
        &source,
        client,
        workspace,
    )?;

    Ok(RunResult { roles: vec![role.clone()], prompt_preview: false, sessions: vec![session_id] })
}

/// Validate that a role exists in the layer's roles directory.
fn validate_role_exists(jules_path: &Path, layer: Layer, role: &str) -> Result<(), AppError> {
    let role_dir = jules_path.join("roles").join(layer.dir_name()).join("roles").join(role);
    let role_yml_path = role_dir.join("role.yml");

    if !role_yml_path.exists() {
        return Err(AppError::RoleNotFound(format!(
            "{}/roles/{} (role.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    Ok(())
}

/// Execute a single role with Jules API.
#[allow(clippy::too_many_arguments)]
fn execute_session<C: JulesClient, L: crate::domain::PromptAssetLoader>(
    jules_path: &Path,
    layer: Layer,
    role: &RoleId,
    workstream: &str,
    starting_branch: &str,
    source: &str,
    client: &C,
    loader: &L,
) -> Result<String, AppError> {
    println!("Executing {} / {}...", layer.dir_name(), role);

    let prompt = assemble_prompt(jules_path, layer, role.as_str(), workstream, loader)?;

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch: starting_branch.to_string(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(response.session_id)
}

/// Execute a prompt preview for a single role.
fn execute_prompt_preview<L: crate::domain::PromptAssetLoader>(
    jules_path: &Path,
    layer: Layer,
    role: &RoleId,
    workstream: &str,
    starting_branch: &str,
    loader: &L,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", layer.display_name());
    println!("Starting branch: {}", starting_branch);
    println!("Workstream: {}", workstream);
    println!("Role: {}\n", role);

    let role_dir =
        jules_path.join("roles").join(layer.dir_name()).join("roles").join(role.as_str());
    let role_yml_path = role_dir.join("role.yml");

    if !role_yml_path.exists() {
        println!("  ⚠️  role.yml not found at {}\n", role_yml_path.display());
        return Ok(());
    }

    let contracts_path = jules_path.join("roles").join(layer.dir_name()).join("contracts.yml");
    let prompt_path = jules_path.join("roles").join(layer.dir_name()).join("prompt.yml");

    println!("  Prompt: {}", prompt_path.display());
    if contracts_path.exists() {
        println!("  Contracts: {}", contracts_path.display());
    }
    println!("  Role config: {}", role_yml_path.display());

    if let Ok(prompt) = assemble_prompt(jules_path, layer, role.as_str(), workstream, loader) {
        println!("  Assembled prompt: {} chars", prompt.len());
    }

    println!("\nWould execute 1 session");
    Ok(())
}
