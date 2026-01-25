//! Update command: update jo-managed docs/templates under `.jules/.jo/`.

use crate::error::AppError;
use crate::workspace::Workspace;

/// Options for the update command.
#[derive(Default)]
pub struct UpdateOptions {
    /// Force overwrite even if local modifications exist.
    pub force: bool,
}

/// Result of the update command.
#[derive(Debug)]
pub struct UpdateResult {
    /// Previous version (if any).
    pub previous_version: Option<String>,
    /// New version.
    pub new_version: String,
}

/// Execute the update command.
pub fn execute(options: &UpdateOptions) -> Result<UpdateResult, AppError> {
    let workspace = Workspace::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let previous_version = workspace.read_version()?;
    let new_version = env!("CARGO_PKG_VERSION").to_string();

    // Check for modifications unless force is set
    if !options.force {
        let modifications = workspace.detect_modifications()?;
        if !modifications.is_empty() {
            return Err(AppError::ModifiedFiles(modifications));
        }
    }

    // Update jo-managed files
    workspace.update_jo_files()?;
    workspace.write_version(&new_version)?;

    Ok(UpdateResult { previous_version, new_version })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::init;
    use std::env;
    use std::fs;
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
    fn update_fails_without_workspace() {
        with_temp_cwd(|| {
            let err = execute(&UpdateOptions::default()).expect_err("update should fail");
            assert!(matches!(err, AppError::WorkspaceNotFound));
        });
    }

    #[test]
    fn update_succeeds_on_clean_workspace() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let result = execute(&UpdateOptions::default()).expect("update should succeed");
            assert_eq!(result.new_version, env!("CARGO_PKG_VERSION"));
        });
    }

    #[test]
    fn update_fails_with_modifications() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            // Modify a jo-managed file
            let cwd = env::current_dir().unwrap();
            let policy = cwd.join(".jules/.jo/policy/contract.md");
            fs::write(&policy, "MODIFIED").unwrap();

            let err = execute(&UpdateOptions::default()).expect_err("update should fail");
            assert!(matches!(err, AppError::ModifiedFiles(_)));
        });
    }

    #[test]
    fn update_force_overwrites_modifications() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            // Modify a jo-managed file
            let cwd = env::current_dir().unwrap();
            let policy = cwd.join(".jules/.jo/policy/contract.md");
            fs::write(&policy, "MODIFIED").unwrap();

            let options = UpdateOptions { force: true };
            execute(&options).expect("update with force should succeed");

            // Verify file was restored
            let content = fs::read_to_string(&policy).unwrap();
            assert!(content.contains("Workspace Contract"));
        });
    }
}
