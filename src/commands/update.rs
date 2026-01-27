//! Update command: update jo-managed docs/templates and role prompts.

use crate::error::AppError;
use crate::workspace::Workspace;

/// Result of the update command.
#[derive(Debug)]
pub struct UpdateResult {
    /// Previous version (if any).
    pub previous_version: Option<String>,
    /// New version.
    pub new_version: String,
    /// Whether any updates were applied.
    pub updated: bool,
}

/// Execute the update command.
pub fn execute() -> Result<UpdateResult, AppError> {
    let workspace = Workspace::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let previous_version = workspace.read_version()?;
    let new_version = env!("CARGO_PKG_VERSION").to_string();

    let modifications = workspace.detect_modifications()?;
    let missing = workspace.missing_managed_files()?;

    // Always update jo-managed files, even if modified
    let up_to_date = previous_version.as_ref() == Some(&new_version)
        && modifications.is_empty()
        && missing.is_empty();
    if up_to_date {
        return Ok(UpdateResult { previous_version, new_version, updated: false });
    }

    // Update jo-managed files and structural scaffolding
    workspace.update_managed_files()?;
    workspace.write_version(&new_version)?;

    Ok(UpdateResult { previous_version, new_version, updated: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::init;
    use serial_test::serial;
    use std::env;
    use std::fs;
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
    fn update_fails_without_workspace() {
        with_temp_cwd(|_dir| {
            let err = execute().expect_err("update should fail");
            assert!(matches!(err, AppError::WorkspaceNotFound));
        });
    }

    #[test]
    #[serial]
    fn update_succeeds_on_clean_workspace() {
        with_temp_cwd(|_dir| {
            init::execute().unwrap();

            let result = execute().expect("update should succeed");
            assert_eq!(result.new_version, env!("CARGO_PKG_VERSION"));
            assert!(!result.updated);
        });
    }

    #[test]
    #[serial]
    fn update_succeeds_with_modifications() {
        with_temp_cwd(|_dir| {
            init::execute().unwrap();

            // Modify a jo-managed file
            let cwd = env::current_dir().unwrap();
            let readme = cwd.join(".jules/README.md");
            fs::write(&readme, "MODIFIED").unwrap();

            let result = execute().expect("update should succeed");
            assert_eq!(result.new_version, env!("CARGO_PKG_VERSION"));
            assert!(result.updated);

            // Verify file was restored
            let content = fs::read_to_string(&readme).unwrap();
            assert_ne!(content, "MODIFIED");
        });
    }
}
