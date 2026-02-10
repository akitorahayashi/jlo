//! API Facade for the application.
//!
//! This module exposes high-level functions that glue together context creation
//! and command execution.

use std::path::{Path, PathBuf};

use crate::adapters::embedded_role_template_store::EmbeddedRoleTemplateStore;
use crate::adapters::git_command::GitCommandAdapter;
use crate::adapters::github_command::GitHubCommandAdapter;
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::app::{
    AppContext,
    commands::{cli_upgrade, create, deinit, doctor, init, run, setup, update},
};
use crate::ports::WorkspaceStore;

pub use crate::app::commands::cli_upgrade::CliUpgradeResult;
pub use crate::app::commands::create::CreateOutcome;
pub use crate::app::commands::deinit::DeinitOutcome;
pub use crate::app::commands::doctor::{DoctorOptions, DoctorOutcome};
pub use crate::app::commands::run::{RunOptions, RunResult};
pub use crate::app::commands::setup::list::{ComponentDetail, ComponentSummary, EnvVarInfo};
pub use crate::app::commands::update::{UpdateOptions, UpdateResult};
pub use crate::app::commands::workflow::{WorkflowBootstrapOptions, WorkflowBootstrapOutput};
pub use crate::domain::AppError;
pub use crate::domain::Layer;
pub use crate::domain::WorkflowRunnerMode;

/// Create an `AppContext` for a given path.
fn create_context(
    path: std::path::PathBuf,
) -> AppContext<FilesystemWorkspaceStore, EmbeddedRoleTemplateStore> {
    let workspace = FilesystemWorkspaceStore::new(path);
    let templates = EmbeddedRoleTemplateStore::new();
    AppContext::new(workspace, templates)
}

/// Initialize a new `.jlo/` control plane and workflow scaffold in the current directory.
pub fn init(mode: WorkflowRunnerMode) -> Result<(), AppError> {
    init_at(std::env::current_dir()?, mode)
}

/// Initialize a new `.jlo/` control plane and workflow scaffold at the specified path.
pub fn init_at(path: impl Into<PathBuf>, mode: WorkflowRunnerMode) -> Result<(), AppError> {
    let path = path.into();
    let ctx = create_context(path.clone());

    let git = GitCommandAdapter::new(path);
    init::execute(&ctx, &git, mode)?;
    Ok(())
}

/// Deinitialize jlo assets from the current directory.
pub fn deinit() -> Result<DeinitOutcome, AppError> {
    deinit_at(std::env::current_dir()?)
}

/// Deinitialize jlo assets from the specified path.
pub fn deinit_at(path: std::path::PathBuf) -> Result<DeinitOutcome, AppError> {
    let git = GitCommandAdapter::new(path.clone());
    deinit::execute(&path, &git)
}

/// Initialize a new workflow scaffold at the specified path (standalone operation).
pub fn init_workflows_at(
    path: std::path::PathBuf,
    mode: WorkflowRunnerMode,
) -> Result<(), AppError> {
    let generate_config = init::load_workflow_generate_config(&path)?;
    init::install_workflow_scaffold(&path, mode, &generate_config)
}

// =============================================================================
// Create Command API
// =============================================================================

/// Create a new role under `.jlo/roles/<layer>/<name>/`.
pub fn create_role(layer: &str, name: &str) -> Result<CreateOutcome, AppError> {
    create_role_at(layer, name, std::env::current_dir()?)
}

/// Create a new role at the specified path.
pub fn create_role_at(
    layer: &str,
    name: &str,
    root: std::path::PathBuf,
) -> Result<CreateOutcome, AppError> {
    let ctx = create_context(root);
    create::create_role(&ctx, layer, name)
}

// =============================================================================
// Run Command API
// =============================================================================

/// Execute Jules agents for a layer.
///
/// # Arguments
/// * `layer` - Target layer (observers, deciders, planners, implementers)
/// * `role` - Specific role to run (required for observers/deciders/innovators)
/// * `prompt_preview` - Show prompts without executing
/// * `branch` - Override the starting branch
/// * `issue` - Local issue file path (required for planners/implementers)
/// * `mock` - Run in mock mode (no Jules API, tag from JULES_MOCK_TAG env)
/// * `phase` - Execution phase for innovators (creation or refinement)
#[allow(clippy::too_many_arguments)]
pub fn run(
    layer: Layer,
    role: Option<String>,
    prompt_preview: bool,
    branch: Option<String>,
    issue: Option<std::path::PathBuf>,
    mock: bool,
    phase: Option<String>,
) -> Result<RunResult, AppError> {
    run_at(layer, role, prompt_preview, branch, issue, mock, phase, std::env::current_dir()?)
}

#[allow(clippy::too_many_arguments)]
pub fn run_at(
    layer: Layer,
    role: Option<String>,
    prompt_preview: bool,
    branch: Option<String>,
    issue: Option<std::path::PathBuf>,
    mock: bool,
    phase: Option<String>,
    root: impl Into<PathBuf>,
) -> Result<RunResult, AppError> {
    let root = root.into();
    let workspace = FilesystemWorkspaceStore::new(root.clone());
    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let git = GitCommandAdapter::new(root);
    let github = GitHubCommandAdapter::new();

    let options = RunOptions { layer, role, prompt_preview, branch, issue, mock, phase };
    run::execute(&workspace.jules_path(), options, &git, &github, &workspace)
}

// =============================================================================
// Setup Compiler API
// =============================================================================

/// Generate setup script and environment configuration.
///
/// Reads `tools.yml`, resolves dependencies, and generates:
/// - `install.sh` - Installation script (executable)
/// - `env.toml` - Environment variables
///
/// Returns the list of resolved component names in installation order.
pub fn setup_gen(path: Option<&Path>) -> Result<Vec<String>, AppError> {
    let store = if let Some(p) = path {
        FilesystemWorkspaceStore::new(p.to_path_buf())
    } else {
        FilesystemWorkspaceStore::current()?
    };
    setup::generate(&store)
}

/// List all available components.
pub fn setup_list() -> Result<Vec<ComponentSummary>, AppError> {
    setup::list()
}

/// Get detailed information for a specific component.
pub fn setup_detail(component: &str) -> Result<ComponentDetail, AppError> {
    setup::list_detail(component)
}

// =============================================================================
// Update Command API
// =============================================================================

/// Update workspace to current jlo version.
///
/// Reconciles the existing workspace with the scaffold embedded in the jlo binary.
/// Only jlo-managed files are overwritten; repository-owned files are preserved.
///
/// # Arguments
/// * `prompt_preview` - Show planned changes without applying
pub fn update(prompt_preview: bool) -> Result<UpdateResult, AppError> {
    update_at(std::env::current_dir()?, prompt_preview)
}

/// Update workspace at the specified path.
pub fn update_at(path: std::path::PathBuf, prompt_preview: bool) -> Result<UpdateResult, AppError> {
    let workspace = FilesystemWorkspaceStore::new(path);
    let templates = EmbeddedRoleTemplateStore::new();
    let options = UpdateOptions { prompt_preview };
    update::execute(&workspace, options, &templates)
}

/// Update the installed jlo CLI binary from the upstream repository.
pub fn update_cli() -> Result<CliUpgradeResult, AppError> {
    cli_upgrade::execute()
}

// =============================================================================
// Doctor Command API
// =============================================================================

/// Validate the `.jules/` workspace structure and content.
pub fn doctor(options: DoctorOptions) -> Result<DoctorOutcome, AppError> {
    doctor_at(std::env::current_dir()?, options)
}

/// Validate the `.jules/` workspace at the specified path.
pub fn doctor_at(
    path: impl Into<PathBuf>,
    options: DoctorOptions,
) -> Result<DoctorOutcome, AppError> {
    let workspace = FilesystemWorkspaceStore::new(path.into());
    doctor::execute(&workspace.jules_path(), options)
}

// =============================================================================
// Workflow Command API
// =============================================================================

/// Materialize `.jules/` from `.jlo/` using the workflow bootstrap process.
pub fn workflow_bootstrap_at(
    path: impl Into<PathBuf>,
) -> Result<WorkflowBootstrapOutput, AppError> {
    let options = WorkflowBootstrapOptions { root: path.into() };
    crate::app::commands::workflow::bootstrap(options)
}
