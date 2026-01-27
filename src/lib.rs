//! jo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

pub mod app;
pub mod domain;
pub mod ports;
pub mod services;

#[cfg(test)]
pub(crate) mod testing;

use app::{
    AppContext,
    commands::{init, template},
};
use ports::{ClipboardWriter, NoopClipboard, WorkspaceStore};
use services::{
    ArboardClipboard, EmbeddedRoleTemplateStore, FilesystemWorkspaceStore, PromptGenerator,
};

pub use domain::AppError;

/// Initialize a new `.jules/` workspace in the current directory.
pub fn init() -> Result<(), AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let ctx = AppContext::new(workspace, templates, NoopClipboard);

    init::execute(&ctx)?;
    println!("✅ Initialized .jules/ workspace with 4-layer architecture");
    Ok(())
}

/// Assign context paths to a role and copy prompt to clipboard.
///
/// Returns the role ID that was matched.
pub fn assign(role_query: &str, paths: &[String]) -> Result<String, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    // Validate workspace exists before clipboard initialization
    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Find role before clipboard initialization
    let role = workspace
        .find_role_fuzzy(role_query)?
        .ok_or_else(|| AppError::RoleNotFound(role_query.to_string()))?;

    // Generate the prompt YAML
    let yaml = PromptGenerator::generate_yaml(&role.id, role.layer, paths)
        .map_err(|e| AppError::config_error(format!("Failed to generate prompt: {}", e)))?;

    // Only now initialize clipboard (when we actually need it)
    let mut clipboard = ArboardClipboard::new()?;
    clipboard.write_text(&yaml)?;

    let message = if paths.is_empty() {
        format!("📋 Copied prompt for '{}' to clipboard", role.id)
    } else {
        format!("📋 Copied prompt for '{}' with {} path(s) to clipboard", role.id, paths.len())
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
