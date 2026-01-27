use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};
use crate::services::PromptGenerator;

/// Execute the assign command.
///
/// Generates a prompt for the specified role with optional path assignments
/// and copies it to the system clipboard.
pub fn execute<W, R, C>(
    ctx: &mut AppContext<W, R, C>,
    role_query: &str,
    paths: &[String],
) -> Result<String, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    if !ctx.workspace().exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Find the role using fuzzy matching
    let role = ctx
        .workspace()
        .find_role_fuzzy(role_query)?
        .ok_or_else(|| AppError::RoleNotFound(role_query.to_string()))?;

    // Generate the prompt YAML
    let yaml = PromptGenerator::generate_yaml(&role.id, role.layer, paths)
        .map_err(|e| AppError::config_error(format!("Failed to generate prompt: {}", e)))?;

    // Copy to clipboard
    ctx.clipboard_mut().write_text(&yaml)?;

    Ok(role.id)
}
