//! Role command: scaffold a role workspace under `.jules/roles/`.

use crate::error::AppError;
use crate::workspace::{Workspace, is_valid_role_id};

/// Options for the role command.
pub struct RoleOptions<'a> {
    /// The role identifier.
    pub role_id: &'a str,
}

/// Execute the role command.
pub fn execute(options: &RoleOptions<'_>) -> Result<(), AppError> {
    let workspace = Workspace::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    if !is_valid_role_id(options.role_id) {
        return Err(AppError::InvalidRoleId(options.role_id.to_string()));
    }

    if workspace.role_exists(options.role_id) {
        // Role already exists - not an error, just skip
        return Ok(());
    }

    workspace.create_role(options.role_id)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::init;
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
    fn role_fails_without_workspace() {
        with_temp_cwd(|| {
            let options = RoleOptions { role_id: "value" };
            let err = execute(&options).expect_err("role should fail");
            assert!(matches!(err, AppError::WorkspaceNotFound));
        });
    }

    #[test]
    fn role_creates_directory() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let options = RoleOptions { role_id: "value" };
            execute(&options).expect("role should succeed");

            let cwd = env::current_dir().unwrap();
            let role_dir = cwd.join(".jules/roles/value");
            assert!(role_dir.exists());
            assert!(role_dir.join("charter.md").exists());
            assert!(role_dir.join("direction.md").exists());
            assert!(role_dir.join("sessions").exists());
        });
    }

    #[test]
    fn role_fails_for_invalid_id() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let options = RoleOptions { role_id: "invalid/id" };
            let err = execute(&options).expect_err("role should fail");
            assert!(matches!(err, AppError::InvalidRoleId(_)));
        });
    }

    #[test]
    fn role_is_idempotent() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let options = RoleOptions { role_id: "value" };
            execute(&options).expect("first role should succeed");
            execute(&options).expect("second role should succeed");
        });
    }
}
