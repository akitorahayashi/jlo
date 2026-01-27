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

    ctx.workspace().write_version(env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
