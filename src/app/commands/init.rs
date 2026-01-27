use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

/// Execute the init command.
pub fn execute<W, R, C>(ctx: &AppContext<W, R, C>) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    if ctx.workspace().exists() {
        return Err(AppError::WorkspaceExists);
    }

    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;

    // Scaffold all built-in roles
    for role_def in ctx.templates().role_definitions() {
        let role_id = crate::domain::RoleId::new(role_def.id)?;
        if !ctx.workspace().role_exists_in_layer(role_def.layer, &role_id) {
            ctx.workspace().scaffold_role_in_layer(
                role_def.layer,
                &role_id,
                role_def.role_yaml,
                Some(role_def.prompt_yaml),
                role_def.has_notes,
            )?;
        }
    }

    ctx.workspace().write_version(env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
