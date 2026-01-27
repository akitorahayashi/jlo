use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

/// Execute the assign command.
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
    let workspace = ctx.workspace();

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Find role
    let role = workspace
        .find_role_fuzzy(role_query)?
        .ok_or_else(|| AppError::RoleNotFound(role_query.to_string()))?;

    // Read the existing prompt.yml from the workspace
    let role_path = workspace
        .role_path(&role)
        .ok_or_else(|| AppError::config_error(format!("Role path not found for {}", role.id)))?;
    let prompt_path = role_path.join("prompt.yml");

    let prompt_content = std::fs::read_to_string(&prompt_path)
        .map_err(|e| AppError::config_error(format!("Failed to read prompt.yml: {}", e)))?;

    // Build the final output: Targets Header + Prompt Content
    let output = if paths.is_empty() {
        prompt_content
    } else {
        let targets = paths.join("\n");
        format!("# Target\n{}\n\n---\n{}", targets, prompt_content)
    };

    ctx.clipboard_mut().write_text(&output)?;

    Ok(role.id)
}
