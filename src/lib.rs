//! jo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

pub mod app;
pub mod domain;
pub mod ports;
pub mod services;

#[cfg(test)]
pub(crate) mod testing;

use app::{
    AppContext,
    commands::{assign, init, template},
};
use services::{ArboardClipboard, EmbeddedRoleTemplateStore, FilesystemWorkspaceStore};

pub use domain::AppError;

/// Initialize a new `.jules/` workspace in the current directory.
pub fn init() -> Result<(), AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let clipboard = ArboardClipboard::new()?;
    let ctx = AppContext::new(workspace, templates, clipboard);

    init::execute(&ctx)?;
    println!("✅ Initialized .jules/ workspace with 4-layer architecture");
    Ok(())
}

/// Assign context paths to a role and copy prompt to clipboard.
///
/// Returns the role ID that was matched.
pub fn assign(role_query: &str, paths: &[String]) -> Result<String, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let clipboard = ArboardClipboard::new()?;
    let mut ctx = AppContext::new(workspace, templates, clipboard);

    let role_id = assign::execute(&mut ctx, role_query, paths)?;
    let message = if paths.is_empty() {
        format!("📋 Copied prompt for '{}' to clipboard", role_id)
    } else {
        format!("📋 Copied prompt for '{}' with {} path(s) to clipboard", role_id, paths.len())
    };
    println!("{}", message);
    Ok(role_id)
}

/// Create a new role from a layer template.
///
/// Returns the full path of the created role (layer/role_name).
pub fn template(layer: Option<&str>, role_name: Option<&str>) -> Result<String, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;
    let templates = EmbeddedRoleTemplateStore::new();
    let clipboard = ArboardClipboard::new()?;
    let ctx = AppContext::new(workspace, templates, clipboard);

    let path = template::execute(&ctx, layer, role_name)?;
    println!("✅ Created new role at .jules/roles/{}/", path);
    Ok(path)
}
