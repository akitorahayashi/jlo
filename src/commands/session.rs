//! Session command: create a new session file under a role's sessions directory.

use crate::error::AppError;
use crate::workspace::Workspace;
use chrono::Utc;
use std::path::PathBuf;

/// Options for the session command.
pub struct SessionOptions<'a> {
    /// The role identifier.
    pub role_id: &'a str,
    /// Optional slug for the session filename.
    pub slug: Option<&'a str>,
}

/// Execute the session command.
///
/// Returns the path to the created session file.
pub fn execute(options: &SessionOptions<'_>) -> Result<PathBuf, AppError> {
    let workspace = Workspace::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    if !workspace.role_exists(options.role_id) {
        return Err(AppError::RoleNotFound(options.role_id.to_string()));
    }

    let now = Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let time = now.format("%H:%M:%S").to_string();
    let slug = options.slug.unwrap_or("session");

    let session_path = workspace.create_session(options.role_id, &date, &time, slug)?;

    Ok(session_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{init, role};
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
    fn session_fails_without_workspace() {
        with_temp_cwd(|| {
            let options = SessionOptions { role_id: "value", slug: None };
            let err = execute(&options).expect_err("session should fail");
            assert!(matches!(err, AppError::WorkspaceNotFound));
        });
    }

    #[test]
    fn session_fails_without_role() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let options = SessionOptions { role_id: "nonexistent", slug: None };
            let err = execute(&options).expect_err("session should fail");
            assert!(matches!(err, AppError::RoleNotFound(_)));
        });
    }

    #[test]
    fn session_creates_file() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();
            role::execute(&role::RoleOptions { role_id: "value" }).unwrap();

            let options = SessionOptions { role_id: "value", slug: Some("test-run") };
            let path = execute(&options).expect("session should succeed");

            assert!(path.exists());
            assert!(path.to_string_lossy().contains("test-run"));
        });
    }

    #[test]
    fn session_uses_default_slug() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();
            role::execute(&role::RoleOptions { role_id: "value" }).unwrap();

            let options = SessionOptions { role_id: "value", slug: None };
            let path = execute(&options).expect("session should succeed");

            assert!(path.exists());
            assert!(path.to_string_lossy().contains("session"));
        });
    }
}
