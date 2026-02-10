//! Run command implementation for executing Jules agents.

mod config;
mod config_dto;
pub mod mock;

pub use config::parse_config_content;
pub mod narrator;
pub(crate) mod narrator_logic;
mod prompt;
pub mod single_role;

use std::path::Path;
use std::path::PathBuf;

use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

use self::config::{detect_repository_source, load_config};
use self::prompt::{assemble_prompt, assemble_single_role_prompt};
use crate::adapters::jules_client_http::HttpJulesClient;
use crate::ports::{AutomationMode, JulesClient, SessionRequest};

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific role to run (required for observers/deciders/innovators).
    pub role: Option<String>,
    /// Show assembled prompts without executing.
    pub prompt_preview: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Local issue file path (required for issue-driven layers: planners, implementers).
    pub issue: Option<PathBuf>,
    /// Run in mock mode (no Jules API, real git/GitHub operations).
    pub mock: bool,
    /// Execution phase for innovators (creation or refinement).
    pub phase: Option<String>,
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
pub fn execute<G, H, W>(
    jules_path: &Path,
    options: RunOptions,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
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
        );
    }

    // Deciders is single-role (no --role required, prompt resolves without role variable)
    if options.layer == Layer::Deciders {
        return execute_decider(jules_path, &options, workspace);
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
        );
    }

    // Observers/Innovators: multi-role execution requiring --role
    execute_multi_role_run(jules_path, &options, workspace)
}

/// Execute the decider layer (single-role, no `--role` required).
fn execute_decider<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
) -> Result<RunResult, AppError> {
    let config = load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    if options.prompt_preview {
        println!("=== Prompt Preview: Decider ===");
        println!("Starting branch: {}\n", starting_branch);

        if let Ok(prompt) = assemble_single_role_prompt(jules_path, Layer::Deciders, workspace) {
            println!("  Assembled prompt: {} chars", prompt.len());
        }

        println!("\nWould dispatch workflow");
        return Ok(RunResult {
            roles: vec!["decider".to_string()],
            prompt_preview: true,
            sessions: vec![],
        });
    }

    let source = detect_repository_source()?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let prompt = assemble_single_role_prompt(jules_path, Layer::Deciders, workspace)?;

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch: starting_branch.to_string(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    println!("Executing: deciders...");
    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(RunResult {
        roles: vec!["decider".to_string()],
        prompt_preview: false,
        sessions: vec![response.session_id],
    })
}

/// Execute a single observer or innovator role.
fn execute_multi_role_run<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
) -> Result<RunResult, AppError> {
    let layer_name = options.layer.dir_name();
    let role = options
        .role
        .as_ref()
        .ok_or_else(|| AppError::MissingArgument(format!("Role is required for {}", layer_name)))?;

    // Validate role exists
    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, options.layer, role_id.as_str())?;

    // Load config
    let config = load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    if options.prompt_preview {
        execute_prompt_preview(
            jules_path,
            options.layer,
            &role_id,
            &starting_branch,
            options.phase.as_deref(),
            workspace,
        )?;
        return Ok(RunResult { roles: vec![role.clone()], prompt_preview: true, sessions: vec![] });
    }

    // Determine repository source from git
    let source = detect_repository_source()?;

    // Execute with Jules API
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = execute_session(
        jules_path,
        options.layer,
        &role_id,
        &starting_branch,
        &source,
        options.phase.as_deref(),
        &client,
        workspace,
    )?;

    Ok(RunResult { roles: vec![role.clone()], prompt_preview: false, sessions: vec![session_id] })
}

/// Validate that a role exists in the layer's roles directory.
fn validate_role_exists(jules_path: &Path, layer: Layer, role: &str) -> Result<(), AppError> {
    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_dir = root.join(".jlo").join("roles").join(layer.dir_name()).join(role);
    let role_yml_path = role_dir.join("role.yml");

    if !role_yml_path.exists() {
        return Err(AppError::RoleNotFound(format!(
            "{}/{} (role.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    Ok(())
}

/// Execute a single role with Jules API.
#[allow(clippy::too_many_arguments)]
fn execute_session<
    C: JulesClient,
    L: crate::domain::PromptAssetLoader + Clone + Send + Sync + 'static,
>(
    jules_path: &Path,
    layer: Layer,
    role: &RoleId,
    starting_branch: &str,
    source: &str,
    phase: Option<&str>,
    client: &C,
    loader: &L,
) -> Result<String, AppError> {
    println!("Executing {} / {}...", layer.dir_name(), role);

    let prompt = assemble_prompt(jules_path, layer, role.as_str(), phase, loader)?;

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
fn execute_prompt_preview<L: crate::domain::PromptAssetLoader + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    layer: Layer,
    role: &RoleId,
    starting_branch: &str,
    phase: Option<&str>,
    loader: &L,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", layer.display_name());
    println!("Starting branch: {}", starting_branch);
    println!("Role: {}\n", role);

    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_dir = root.join(".jlo").join("roles").join(layer.dir_name()).join(role.as_str());
    let role_yml_path = role_dir.join("role.yml");

    if !role_yml_path.exists() {
        println!("  ⚠️  role.yml not found at {}\n", role_yml_path.display());
        return Ok(());
    }

    if layer == Layer::Innovators {
        if let Some(p) = phase {
            let contracts_path = jules_path
                .join("roles")
                .join(layer.dir_name())
                .join(format!("contracts_{}.yml", p));
            if contracts_path.exists() {
                println!("  Contracts: {}", contracts_path.display());
            }
        }
    } else {
        // contracts.yml is a runtime artifact in .jules/
        let contracts_path = jules_path.join("roles").join(layer.dir_name()).join("contracts.yml");
        if contracts_path.exists() {
            println!("  Contracts: {}", contracts_path.display());
        }
    }
    println!("  Role config: {}", role_yml_path.display());

    if let Ok(prompt) = assemble_prompt(jules_path, layer, role.as_str(), phase, loader) {
        println!("  Assembled prompt: {} chars", prompt.len());
    }

    println!("\nWould execute 1 session");
    Ok(())
}
