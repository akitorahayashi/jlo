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
    commands::{init, run, setup, template},
};
use domain::Layer;
use ports::{NoopClipboard, WorkspaceStore};
use services::{EmbeddedRoleTemplateStore, FilesystemWorkspaceStore};

pub use app::commands::run::execute::{RunOptions, RunResult};
pub use app::commands::setup::list::{ComponentDetail, ComponentSummary, EnvVarInfo};
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

/// Create a new role from a layer template.
///
/// Returns the full path of the created role (layer/role_name).
pub fn template(layer: Option<&str>, role_name: Option<&str>) -> Result<String, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let ctx = AppContext::new(workspace, templates, NoopClipboard);

    let path = template::execute(&ctx, layer, role_name)?;
    println!("✅ Created new role at .jules/roles/{}/", path);
    Ok(path)
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
pub fn run(
    layer: Layer,
    roles: Option<Vec<String>>,
    dry_run: bool,
    branch: Option<String>,
) -> Result<RunResult, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let options = RunOptions { layer, roles, dry_run, branch };
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
