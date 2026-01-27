//! jo: Deploy and manage .jules/ workspace scaffolding for organizational memory.

mod commands;
pub mod error;
mod generator;
mod layers;
mod scaffold;
mod templates;
mod workspace;

use commands::{assign, init, template};
use error::AppError;

/// Initialize a new `.jules/` workspace in the current directory.
pub fn init() -> Result<(), AppError> {
    init::execute()?;
    println!("✅ Initialized .jules/ workspace with 4-layer architecture");
    Ok(())
}

/// Assign context paths to a role and copy prompt to clipboard.
///
/// Returns the role ID that was matched.
pub fn assign(role_query: &str, paths: &[String]) -> Result<String, AppError> {
    let role_id = assign::execute(role_query, paths)?;
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
    let path = template::execute(layer, role_name)?;
    println!("✅ Created new role at .jules/roles/{}/", path);
    Ok(path)
}
