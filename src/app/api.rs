//! API Facade for the application.
//!
//! This module exposes high-level functions that glue together context creation
//! and command execution.

use std::path::{Path, PathBuf};

use crate::adapters::catalogs::EmbeddedRoleTemplateStore;
use crate::adapters::git::GitCommandAdapter;
use crate::adapters::github::GitHubCommandAdapter;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::app::{
    AppContext,
    commands::{add, cli_upgrade, create, deinit, doctor, init, run, setup, update},
};
use crate::ports::{JulesStore, RoleTemplateStore};

pub use crate::app::commands::add::AddOutcome;
pub use crate::app::commands::cli_upgrade::CliUpgradeResult;
pub use crate::app::commands::create::CreateOutcome;
pub use crate::app::commands::deinit::DeinitOutcome;
pub use crate::app::commands::doctor::{DoctorOptions, DoctorOutcome};
pub use crate::app::commands::run::{RunOptions, RunResult};
pub use crate::app::commands::setup::list::{
    EnvVarInfo, SetupComponentDetail, SetupComponentSummary,
};
pub use crate::app::commands::update::{UpdateOptions, UpdateResult};
pub use crate::app::commands::workflow::{
    WorkflowBootstrapManagedFilesOutput, WorkflowBootstrapWorkstationsOutput,
};
pub use crate::domain::AppError;
pub use crate::domain::WorkflowRunnerMode;
pub use crate::domain::{BuiltinRoleEntry, Layer};

/// Create an `AppContext` for a given path.
fn create_context(
    path: std::path::PathBuf,
) -> AppContext<LocalRepositoryAdapter, EmbeddedRoleTemplateStore> {
    let repository = LocalRepositoryAdapter::new(path);
    let templates = EmbeddedRoleTemplateStore::new();
    AppContext::new(repository, templates)
}

/// Initialize a new `.jlo/` control plane and workflow scaffold in the current directory.
pub fn init(mode: &WorkflowRunnerMode) -> Result<(), AppError> {
    init_at(std::env::current_dir()?, mode)
}

/// Initialize a new `.jlo/` control plane and workflow scaffold at the specified path.
pub fn init_at(path: impl Into<PathBuf>, mode: &WorkflowRunnerMode) -> Result<(), AppError> {
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
    mode: &WorkflowRunnerMode,
) -> Result<(), AppError> {
    let repository = LocalRepositoryAdapter::new(path.clone());
    let generate_config =
        crate::adapters::control_plane_config::load_workflow_generate_config(&repository)?;
    crate::adapters::workflow_installer::install_workflow_scaffold(
        &repository,
        mode,
        &generate_config,
    )
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
// Add Command API
// =============================================================================

/// Register a built-in role in `.jlo/config.toml`.
pub fn add_role(layer: &str, name: &str) -> Result<AddOutcome, AppError> {
    add_role_at(layer, name, std::env::current_dir()?)
}

/// Install a built-in role at the specified path.
pub fn add_role_at(
    layer: &str,
    name: &str,
    root: std::path::PathBuf,
) -> Result<AddOutcome, AppError> {
    let ctx = create_context(root);
    add::add_role(&ctx, layer, name)
}

/// List the built-in role catalog.
pub fn builtin_role_catalog() -> Result<Vec<BuiltinRoleEntry>, AppError> {
    let store = EmbeddedRoleTemplateStore::new();
    store.builtin_role_catalog()
}

// =============================================================================
// Run Command API
// =============================================================================

/// Execute Jules agents for a layer.
///
/// # Arguments
/// * `layer` - Target layer (observers, decider, planner, implementer, integrator)
/// * `role` - Specific role to run (required for observers/decider/innovators)
/// * `prompt_preview` - Show prompts without executing
/// * `branch` - Override the starting branch
/// * `requirement` - Local requirement file path (required for planner/implementer)
/// * `mock` - Run in mock mode (no Jules API, tag from JULES_MOCK_TAG env)
/// * `task` - Innovator task selector (expected: create_three_proposals)
#[allow(clippy::too_many_arguments)]
pub fn run(
    layer: Layer,
    role: Option<String>,
    prompt_preview: bool,
    branch: Option<String>,
    requirement: Option<std::path::PathBuf>,
    mock: bool,
    task: Option<String>,
    no_cleanup: bool,
) -> Result<RunResult, AppError> {
    run_at(
        layer,
        role,
        prompt_preview,
        branch,
        requirement,
        mock,
        task,
        no_cleanup,
        std::env::current_dir()?,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_at(
    layer: Layer,
    role: Option<String>,
    prompt_preview: bool,
    branch: Option<String>,
    requirement: Option<std::path::PathBuf>,
    mock: bool,
    task: Option<String>,
    no_cleanup: bool,
    root: impl Into<PathBuf>,
) -> Result<RunResult, AppError> {
    let root = root.into();
    let repository = LocalRepositoryAdapter::new(root.clone());
    if !repository.jules_exists() {
        return Err(AppError::JulesNotFound);
    }

    let git = GitCommandAdapter::new(root);
    let github = GitHubCommandAdapter::new();

    let options =
        RunOptions { layer, role, prompt_preview, branch, requirement, mock, task, no_cleanup };
    run::execute(&repository.jules_path(), options, &git, &github, &repository)
}

// =============================================================================
// Setup Compiler API
// =============================================================================

/// Generate setup script and environment configuration.
///
/// Reads `tools.yml`, resolves dependencies, and generates:
/// - `install.sh` - Installation script (executable)
/// - `vars.toml` - Non-secret environment variables
/// - `secrets.toml` - Secret environment variables
///
/// Returns the list of resolved component names in installation order.
pub fn setup_gen(path: Option<&Path>) -> Result<Vec<String>, AppError> {
    let store = if let Some(p) = path {
        LocalRepositoryAdapter::new(p.to_path_buf())
    } else {
        LocalRepositoryAdapter::current()?
    };
    setup::generate(&store)
}

/// List all available components.
pub fn setup_list() -> Result<Vec<SetupComponentSummary>, AppError> {
    setup::list()
}

/// Get detailed information for a specific component.
pub fn setup_detail(component: &str) -> Result<SetupComponentDetail, AppError> {
    setup::list_detail(component)
}

// =============================================================================
// Update Command API
// =============================================================================

/// Update repository to current jlo version.
///
/// Reconciles the existing repository with the scaffold embedded in the jlo binary.
/// Only jlo-managed files are overwritten; repository-owned files are preserved.
///
/// # Arguments
/// * `prompt_preview` - Show planned changes without applying
pub fn update(prompt_preview: bool) -> Result<UpdateResult, AppError> {
    update_at(std::env::current_dir()?, prompt_preview)
}

/// Update repository at the specified path.
pub fn update_at(path: std::path::PathBuf, prompt_preview: bool) -> Result<UpdateResult, AppError> {
    let repository = LocalRepositoryAdapter::new(path);
    let templates = EmbeddedRoleTemplateStore::new();
    let options = UpdateOptions { prompt_preview };
    update::execute(&repository, options, &templates)
}

/// Update the installed jlo CLI binary from the upstream repository.
pub fn update_cli() -> Result<CliUpgradeResult, AppError> {
    cli_upgrade::execute()
}

// =============================================================================
// Doctor Command API
// =============================================================================

/// Validate the `.jules/` repository structure and content.
pub fn doctor(options: DoctorOptions) -> Result<DoctorOutcome, AppError> {
    doctor_at(std::env::current_dir()?, options)
}

/// Validate the `.jules/` repository at the specified path.
pub fn doctor_at(
    path: impl Into<PathBuf>,
    options: DoctorOptions,
) -> Result<DoctorOutcome, AppError> {
    let repository = LocalRepositoryAdapter::new(path.into());
    doctor::execute(&repository.jules_path(), options)
}

// =============================================================================
// Workflow Command API
// =============================================================================

/// Materialize managed runtime files under `.jules/` from embedded scaffold assets.
pub fn workflow_bootstrap_managed_files_at(
    path: impl Into<PathBuf>,
) -> Result<WorkflowBootstrapManagedFilesOutput, AppError> {
    let options =
        crate::app::commands::workflow::WorkflowBootstrapManagedFilesOptions { root: path.into() };
    crate::app::commands::workflow::bootstrap_managed_files(options)
}

/// Reconcile workstation perspectives under `.jules/workstations/`.
pub fn workflow_bootstrap_workstations_at(
    path: impl Into<PathBuf>,
) -> Result<WorkflowBootstrapWorkstationsOutput, AppError> {
    let options =
        crate::app::commands::workflow::WorkflowBootstrapWorkstationsOptions { root: path.into() };
    crate::app::commands::workflow::bootstrap_workstations(options)
}
