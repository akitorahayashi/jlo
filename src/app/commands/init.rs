use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{RoleTemplateStore, WorkspaceStore};
use crate::services::{manifest_from_scaffold, write_manifest};

/// Execute the init command.
///
/// Creates both the `.jules/` workspace and `.jules/setup/` directory.
pub fn execute<W, R>(ctx: &AppContext<W, R>) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    if ctx.workspace().exists() {
        return Err(AppError::WorkspaceExists);
    }

    // Enforce execution on 'jules' branch to protect main history
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map_err(|e| AppError::ConfigError(format!("Failed to run git to check branch: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::ConfigError(format!(
            "Failed to get current git branch. Is this a git repository?\nDetails: {stderr}"
        )));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch != "jules" {
        return Err(AppError::ConfigError(format!(
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
