//! Init command: create `.jules/` skeleton and source-of-truth docs.

use crate::error::AppError;
use crate::workspace::Workspace;

/// Options for the init command.
#[derive(Default)]
pub struct InitOptions {
    /// Force initialization even if workspace exists.
    pub force: bool,
}

/// Execute the init command.
pub fn execute(options: &InitOptions) -> Result<(), AppError> {
    let workspace = Workspace::current()?;

    if workspace.exists() && !options.force {
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
        F: FnOnce() -> R,
    {
        let dir = TempDir::new().expect("failed to create temp dir");
        let original = env::current_dir().expect("failed to get cwd");
        env::set_current_dir(dir.path()).expect("failed to set cwd");
        let result = f();
        env::set_current_dir(original).expect("failed to restore cwd");
        result
    }

    #[test]
    #[serial]
    fn init_creates_workspace() {
        with_temp_cwd(|| {
            let options = InitOptions::default();
            execute(&options).expect("init should succeed");

            let cwd = env::current_dir().unwrap();
            assert!(cwd.join(".jules").exists());
            assert!(cwd.join(".jules/.jo-version").exists());
        });
    }

    #[test]
    #[serial]
    fn init_fails_if_exists_without_force() {
        with_temp_cwd(|| {
            let options = InitOptions::default();
            execute(&options).expect("first init should succeed");

            let err = execute(&options).expect_err("second init should fail");
            assert!(matches!(err, AppError::WorkspaceExists));
        });
    }

    #[test]
    #[serial]
    fn init_succeeds_with_force() {
        with_temp_cwd(|| {
            execute(&InitOptions::default()).expect("first init should succeed");

            let options = InitOptions { force: true };
            execute(&options).expect("init with force should succeed");
        });
    }
}
