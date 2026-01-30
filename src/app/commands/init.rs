use crate::app::AppContext;
use crate::domain::AppError;
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

/// Execute the init command.
///
/// Creates both the `.jules/` workspace and `.jules/setup/` directory.
pub fn execute<W, R, C>(ctx: &AppContext<W, R, C>) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    if ctx.workspace().exists() {
        return Err(AppError::WorkspaceExists);
    }

    // Enforce execution on 'jules' branch to protect main history
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map_err(|_| AppError::ConfigError("Failed to check git branch".to_string()))?;

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

    Ok(())
}
