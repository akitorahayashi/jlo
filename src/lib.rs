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
use services::{ArboardClipboard, EmbeddedRoleTemplateStore, FilesystemWorkspaceStore};

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

    // Read the existing prompt.yml from the workspace
    let prompt_path = workspace
        .role_path(&role)
        .ok_or_else(|| AppError::config_error("Role path not found"))?
        .join("prompt.yml");

    let prompt_content = std::fs::read_to_string(&prompt_path)
        .map_err(|e| AppError::config_error(format!("Failed to read prompt.yml: {}", e)))?;

    // Parse the YAML
    let mut prompt_yaml: serde_yaml::Value = serde_yaml::from_str(&prompt_content)
        .map_err(|e| AppError::config_error(format!("Failed to parse prompt.yml: {}", e)))?;

    // Add paths if provided by user at command line
    if !paths.is_empty()
        && let Some(mapping) = prompt_yaml.as_mapping_mut()
    {
        let paths_value = serde_yaml::Value::Sequence(
            paths.iter().map(|p| serde_yaml::Value::String(p.clone())).collect(),
        );
        mapping.insert(serde_yaml::Value::String("paths".to_string()), paths_value);
    }

    // Serialize back to YAML
    let yaml = serde_yaml::to_string(&prompt_yaml)
        .map_err(|e| AppError::config_error(format!("Failed to serialize prompt: {}", e)))?;

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
