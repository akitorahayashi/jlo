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
    commands::{init, prune, setup, template},
};
use ports::{ClipboardWriter, NoopClipboard, WorkspaceStore};
use services::{EmbeddedRoleTemplateStore, FilesystemWorkspaceStore};

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

/// Assign context paths to a role and copy prompt to clipboard.
///
/// Returns the role ID that was matched.
pub fn assign(role_query: &str, paths: &[String]) -> Result<String, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();

    // Use NoopClipboard for validation phase
    let ctx = AppContext::new(workspace, templates, NoopClipboard);

    // Perform validation without clipboard
    if !ctx.workspace().exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let role = ctx
        .workspace()
        .find_role_fuzzy(role_query)?
        .ok_or_else(|| AppError::RoleNotFound(role_query.to_string()))?;

    let role_path = ctx
        .workspace()
        .role_path(&role)
        .ok_or_else(|| AppError::config_error(format!("Role path not found for {}", role.id)))?;
    let prompt_path = role_path.join("prompt.yml");

    let prompt_content = std::fs::read_to_string(&prompt_path)
        .map_err(|e| AppError::config_error(format!("Failed to read prompt.yml: {}", e)))?;

    let output = if paths.is_empty() {
        prompt_content
    } else {
        let targets = paths.join("\n");
        format!("# Target\n{}\n\n---\n{}", targets, prompt_content)
    };

    // Only initialize clipboard after validation succeeds
    let mut clipboard = crate::services::ArboardClipboard::new()?;
    clipboard.write_text(&output)?;

    let message = if paths.is_empty() {
        format!("📋 Copied prompt for '{}' to clipboard", role.id)
    } else {
        format!("📋 Copied prompt for '{}' with {} target(s) to clipboard", role.id, paths.len())
    };
    println!("{}", message);
    Ok(role.id)
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

/// Prune old jules/* branches from remote.
pub fn prune(days: u32, dry_run: bool) -> Result<(), AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let ctx = AppContext::new(workspace, templates, NoopClipboard);

    prune::execute(&ctx, days, dry_run)
}

// =============================================================================
// Setup Compiler API
// =============================================================================

/// Initialize setup workspace at `.jules/setup/`.
///
/// Creates the directory structure with `tools.yml` template.
pub fn setup_init(path: Option<&Path>) -> Result<(), AppError> {
    setup::init(path)
}

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
