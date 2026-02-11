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

use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::prompt_assembly::{innovators as innovators_asm, observers as observers_asm};
use crate::domain::workspace::paths::{jlo, jules};
use crate::domain::{AppError, Layer, RoleId};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

use self::config::{detect_repository_source, load_config};
use self::prompt::assemble_single_role_prompt;
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

    // Validate phase if provided (prevents path traversal)
    if let Some(ref phase) = options.phase
        && !validate_safe_path_component(phase)
    {
        return Err(AppError::Validation(format!(
            "Invalid phase '{}': must be a safe path component (e.g. 'creation', 'refinement')",
            phase,
        )));
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

    // Layer-specific multi-role execution
    match options.layer {
        Layer::Observers => execute_observer_run(jules_path, &options, workspace),
        Layer::Innovators => execute_innovator_run(jules_path, &options, workspace),
        _ => Err(AppError::Validation(format!(
            "Unexpected layer '{}' reached multi-role dispatch",
            options.layer.dir_name()
        ))),
    }
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

/// Execute a single observer role.
fn execute_observer_run<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
) -> Result<RunResult, AppError> {
    let role = options
        .role
        .as_ref()
        .ok_or_else(|| AppError::MissingArgument("Role is required for observers".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Observers, role_id.as_str())?;

    let config = load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    // Resolve observer-specific context: bridge_task
    let bridge_task = resolve_observer_bridge_task(jules_path, workspace);
    let input =
        observers_asm::ObserverPromptInput { role: role_id.as_str(), bridge_task: &bridge_task };

    if options.prompt_preview {
        print_role_preview(jules_path, Layer::Observers, &role_id, &starting_branch);
        if let Ok(assembled) = observers_asm::assemble(jules_path, &input, workspace) {
            println!("  Assembled prompt: {} chars", assembled.content.len());
        }
        println!("\nWould execute 1 session");
        return Ok(RunResult { roles: vec![role.clone()], prompt_preview: true, sessions: vec![] });
    }

    let source = detect_repository_source()?;
    let assembled = observers_asm::assemble(jules_path, &input, workspace)?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = dispatch_session(
        Layer::Observers,
        &role_id,
        assembled.content,
        &source,
        &starting_branch,
        &client,
    )?;

    Ok(RunResult { roles: vec![role.clone()], prompt_preview: false, sessions: vec![session_id] })
}

/// Execute a single innovator role.
fn execute_innovator_run<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
) -> Result<RunResult, AppError> {
    let role = options
        .role
        .as_ref()
        .ok_or_else(|| AppError::MissingArgument("Role is required for innovators".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Innovators, role_id.as_str())?;

    let config = load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    // Resolve innovator-specific context: phase → task file
    let phase = options.phase.as_deref().ok_or_else(|| {
        AppError::MissingArgument(
            "--phase is required for innovators (creation or refinement)".to_string(),
        )
    })?;
    let task_content = resolve_innovator_task(jules_path, phase, workspace);
    let input =
        innovators_asm::InnovatorPromptInput { role: role_id.as_str(), phase, task: &task_content };

    if options.prompt_preview {
        print_role_preview(jules_path, Layer::Innovators, &role_id, &starting_branch);
        if let Ok(assembled) = innovators_asm::assemble(jules_path, &input, workspace) {
            println!("  Assembled prompt: {} chars", assembled.content.len());
        }
        println!("\nWould execute 1 session");
        return Ok(RunResult { roles: vec![role.clone()], prompt_preview: true, sessions: vec![] });
    }

    let source = detect_repository_source()?;
    let assembled = innovators_asm::assemble(jules_path, &input, workspace)?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = dispatch_session(
        Layer::Innovators,
        &role_id,
        assembled.content,
        &source,
        &starting_branch,
        &client,
    )?;

    Ok(RunResult { roles: vec![role.clone()], prompt_preview: false, sessions: vec![session_id] })
}

/// Print common prompt-preview header for multi-role layers.
fn print_role_preview(jules_path: &Path, layer: Layer, role: &RoleId, starting_branch: &str) {
    println!("=== Prompt Preview: {} ===", layer.display_name());
    println!("Starting branch: {}", starting_branch);
    println!("Role: {}\n", role);

    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_yml_path = jlo::role_yml(root, layer, role.as_str());

    if !role_yml_path.exists() {
        println!("  ⚠️  role.yml not found at {}\n", role_yml_path.display());
        return;
    }

    let contracts_path = jules::contracts(jules_path, layer);
    if contracts_path.exists() {
        println!("  Contracts: {}", contracts_path.display());
    }
    println!("  Role config: {}", role_yml_path.display());
}

/// Read bridge_comments.yml content if any innovator persona has an idea.yml.
/// Returns empty string if no ideas exist.
fn resolve_observer_bridge_task<W: WorkspaceStore>(jules_path: &Path, workspace: &W) -> String {
    let innovators = jules::innovators_dir(jules_path);

    let has_ideas = std::fs::read_dir(&innovators)
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                entry.file_type().ok().is_some_and(|ft| ft.is_dir())
                    && entry.path().join("idea.yml").exists()
            })
        })
        .unwrap_or(false);

    if !has_ideas {
        return String::new();
    }

    let bridge_path = jules::tasks_dir(jules_path, Layer::Observers).join("bridge_comments.yml");
    workspace.read_file(&bridge_path.to_string_lossy()).unwrap_or_default()
}

/// Read the task file content for the current innovator phase.
/// Maps phase to task filename: creation → create_idea.yml, refinement → refine_proposal.yml.
fn resolve_innovator_task<W: WorkspaceStore>(
    jules_path: &Path,
    phase: &str,
    workspace: &W,
) -> String {
    let filename = match phase {
        "creation" => "create_idea.yml",
        "refinement" => "refine_proposal.yml",
        _ => return String::new(),
    };
    let task_path = jules::tasks_dir(jules_path, Layer::Innovators).join(filename);
    workspace.read_file(&task_path.to_string_lossy()).unwrap_or_default()
}

/// Validate that a role exists in the layer's roles directory.
fn validate_role_exists(jules_path: &Path, layer: Layer, role: &str) -> Result<(), AppError> {
    let root = jules_path.parent().unwrap_or(Path::new("."));
    let role_yml_path = jlo::role_yml(root, layer, role);

    if !role_yml_path.exists() {
        return Err(AppError::RoleNotFound(format!(
            "{}/{} (role.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    Ok(())
}

/// Dispatch a pre-assembled prompt to the Jules API.
fn dispatch_session<C: JulesClient>(
    layer: Layer,
    role: &RoleId,
    prompt: String,
    source: &str,
    starting_branch: &str,
    client: &C,
) -> Result<String, AppError> {
    println!("Executing {} / {}...", layer.dir_name(), role);

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
