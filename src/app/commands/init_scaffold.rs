use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{GitPort, RoleTemplateStore, WorkspaceStore};
use crate::services::assets::scaffold_manifest::{manifest_from_scaffold, write_manifest};

/// Execute the init command.
///
/// Creates both the `.jules/` workspace and `.jules/setup/` directory.
pub fn execute<W, R, G>(ctx: &AppContext<W, R>, git: &G) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    G: GitPort,
{
    if ctx.workspace().exists() {
        return Err(AppError::WorkspaceExists);
    }

    // Enforce execution on 'jules' branch to protect main history
    let branch = git.get_current_branch()?;

    if branch != "jules" {
        return Err(AppError::Validation(format!(
            "Init must be run on 'jules' branch (current: '{}').\nPlease run: git checkout -b jules",
            branch
        )));
    }

    let scaffold_files = ctx.templates().scaffold_files();
    ctx.workspace().create_structure(&scaffold_files)?;

    ctx.workspace().write_version(env!("CARGO_PKG_VERSION"))?;
    let managed_manifest = manifest_from_scaffold(&scaffold_files);
    write_manifest(&ctx.workspace().jules_path(), &managed_manifest)?;

    Ok(())
}
