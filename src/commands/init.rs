//! Init command: create the `.jules/` workspace structure.

use crate::error::AppError;
use crate::workspace::Workspace;

/// Execute the init command.
pub fn execute() -> Result<(), AppError> {
    let workspace = Workspace::current()?;

    if workspace.exists() {
        return Err(AppError::WorkspaceExists);
    }

    workspace.create_structure()?;
    workspace.write_version(env!("CARGO_PKG_VERSION"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    fn with_temp_cwd<F, R>(f: F) -> R
    where
        F: FnOnce(&TempDir) -> R,
    {
        let dir = TempDir::new().expect("failed to create temp dir");
        let original = env::current_dir().expect("failed to get cwd");
        env::set_current_dir(dir.path()).expect("failed to set cwd");
        let result = f(&dir);
        env::set_current_dir(&original).expect("failed to restore cwd");
        result
    }

    #[test]
    #[serial]
    fn init_creates_workspace() {
        with_temp_cwd(|_dir| {
            execute().expect("init should succeed");

            let cwd = env::current_dir().unwrap();
            assert!(cwd.join(".jules").exists());
            assert!(cwd.join(".jules/.jo-version").exists());
        });
    }

    #[test]
    #[serial]
    fn init_fails_if_exists() {
        with_temp_cwd(|_dir| {
            execute().expect("first init should succeed");

            let err = execute().expect_err("second init should fail");
            assert!(matches!(err, AppError::WorkspaceExists));
        });
    }
}
