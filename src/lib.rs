//! jlo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

pub mod app;
pub mod domain;
pub mod ports;
pub mod services;

#[cfg(test)]
pub(crate) mod testing;

use std::path::Path;

use app::{
    AppContext,
    commands::{init, run, setup, template, update},
};
use domain::Layer;
use ports::{NoopClipboard, WorkspaceStore};
use services::{EmbeddedRoleTemplateStore, FilesystemWorkspaceStore};

pub use app::commands::run::{RunOptions, RunResult};
pub use app::commands::setup::list::{ComponentDetail, ComponentSummary, EnvVarInfo};
pub use app::commands::template::TemplateOutcome;
pub use app::commands::update::{UpdateOptions, UpdateResult};
pub use domain::AppError;

/// Initialize a new `.jules/` workspace in the current directory.
pub fn init() -> Result<(), AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let ctx = AppContext::new(workspace, templates, NoopClipboard);

    init::execute(&ctx)?;
    println!("✅ Initialized .jules/ workspace");
    Ok(())
}

/// Apply a template for a role or workstream.
///
/// Returns a `TemplateOutcome` describing the created resource.
pub fn template(
    layer: Option<&str>,
    role_name: Option<&str>,
    workstream: Option<&str>,
) -> Result<TemplateOutcome, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let ctx = AppContext::new(workspace, templates, NoopClipboard);

    let outcome = template::execute(&ctx, layer, role_name, workstream)?;
    match &outcome {
        TemplateOutcome::Role { .. } => {
            println!("✅ Created new role at {}/", outcome.display_path());
        }
        TemplateOutcome::Workstream { .. } => {
            println!("✅ Created new workstream at {}/", outcome.display_path());
        }
    }
    Ok(outcome)
}

// =============================================================================
// Run Command API
// =============================================================================

/// Execute Jules agents for a layer.
///
/// Runs agents defined in `.jules/config.toml` for the specified layer.
///
/// # Arguments
/// * `layer` - Target layer (observers, deciders, planners, implementers)
/// * `roles` - Specific roles to run (None = all from config)
/// * `dry_run` - Show prompts without executing
/// * `branch` - Override the starting branch
/// * `issue` - Local issue file path (required for implementers)
pub fn run(
    layer: Layer,
    roles: Option<Vec<String>>,
    dry_run: bool,
    branch: Option<String>,
    issue: Option<std::path::PathBuf>,
) -> Result<RunResult, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let options = RunOptions { layer, roles, dry_run, branch, issue };
    run::execute(&workspace.jules_path(), options)
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
    setup::generate(path)
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
/// * `dry_run` - Show planned changes without applying
/// * `workflows` - Include workflow files in update
pub fn update(dry_run: bool, workflows: bool) -> Result<UpdateResult, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let options = UpdateOptions { dry_run, workflows };
    update::execute(&workspace.jules_path(), options)
}
