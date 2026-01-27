use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};
use crate::services::ArboardClipboard;

/// Execute the assign command.
pub fn execute<W, R, C>(
    ctx: &AppContext<W, R, C>,
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

    // Initialize real clipboard for this interactive command
    // We bypass the mockable C here because we specifically want system clipboard for `assign`
    // In a pure architecture, we might want to use C, but currently `assign` logic in lib.rs
    // was using ArboardClipboard directly. We'll stick to that pattern for now to match behavior,
    // or better, use C if C is indeed the clipboard writer.
    // However, AppContext's C might be NoopClipboard in `lib.rs::init` context.
    // But `assign` is an interactive user command.

    // Let's check `lib.rs`: it initializes ArboardClipboard inside `assign`.
    // We should probably rely on `ctx` having a real clipboard if possible,
    // but `jo::assign` instantiates a fresh workspace/clipboard.

    let mut clipboard = ArboardClipboard::new()?;
    clipboard.write_text(&output)?;

    Ok(role.id)
}
