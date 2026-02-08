//! Create workstream under `.jlo/workstreams/<name>/`.

use crate::app::AppContext;
use crate::domain::AppError;
use crate::domain::identities::validation::validate_safe_path_component;
use crate::ports::{RoleTemplateStore, WorkspaceStore};

use super::CreateOutcome;

pub fn execute<W, R>(ctx: &AppContext<W, R>, name: &str) -> Result<CreateOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    if !ctx.workspace().jlo_exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    if !validate_safe_path_component(name) {
        return Err(AppError::Validation(format!(
            "Invalid workstream name '{}'. Use alphanumeric characters, hyphens, or underscores.",
            name
        )));
    }

    let ws_dir = ctx.workspace().jlo_path().join("workstreams").join(name);
    if ws_dir.exists() {
        return Err(AppError::Validation(format!(
            "Workstream '{}' already exists at {}",
            name,
            ws_dir.display()
        )));
    }

    // Seed with default scheduled.toml from workstream templates
    let scheduled_content =
        crate::adapters::assets::workstream_template_assets::workstream_template_content(
            "scheduled.toml",
        )?;

    std::fs::create_dir_all(&ws_dir)?;
    std::fs::write(ws_dir.join("scheduled.toml"), scheduled_content)?;

    Ok(CreateOutcome::Workstream { name: name.to_string() })
}
