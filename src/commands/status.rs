//! Status command: print version info and detect local modifications.

use crate::error::AppError;
use crate::workspace::Workspace;

/// Status information for a workspace.
pub struct StatusResult {
    /// Whether a .jules/ workspace exists.
    pub workspace_exists: bool,
    /// Installed jo version.
    pub installed_version: String,
    /// Workspace version from .jo-version (if exists).
    pub workspace_version: Option<String>,
    /// List of modified jo-managed files.
    pub modified_files: Vec<String>,
    /// Whether an update is available.
    pub update_available: bool,
}

/// Execute the status command.
pub fn execute() -> Result<StatusResult, AppError> {
    let workspace = Workspace::current()?;
    let installed_version = env!("CARGO_PKG_VERSION").to_string();

    if !workspace.exists() {
        return Ok(StatusResult {
            workspace_exists: false,
            installed_version,
            workspace_version: None,
            modified_files: Vec::new(),
            update_available: false,
        });
    }

    let workspace_version = workspace.read_version()?;
    let modified_files = workspace.detect_modifications()?;

    let update_available =
        workspace_version.as_ref().map(|v| v != &installed_version).unwrap_or(true);

    Ok(StatusResult {
        workspace_exists: true,
        installed_version,
        workspace_version,
        modified_files,
        update_available,
    })
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
    fn status_reports_no_workspace() {
        with_temp_cwd(|| {
            let result = execute().expect("status should succeed");
            assert!(!result.workspace_exists);
            assert!(result.workspace_version.is_none());
        });
    }

    #[test]
    #[serial]
    fn status_reports_workspace_info() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            let result = execute().expect("status should succeed");
            assert!(result.workspace_exists);
            assert!(result.workspace_version.is_some());
            assert!(!result.update_available);
        });
    }

    #[test]
    #[serial]
    fn status_detects_modifications() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            // Modify a jo-managed file
            let cwd = env::current_dir().unwrap();
            let policy = cwd.join(".jules/.jo/policy/contract.md");
            fs::write(&policy, "MODIFIED").unwrap();

            let result = execute().expect("status should succeed");
            assert!(!result.modified_files.is_empty());
        });
    }

    #[test]
    #[serial]
    fn status_detects_version_mismatch() {
        with_temp_cwd(|| {
            init::execute(&init::InitOptions::default()).unwrap();

            // Simulate older version
            let cwd = env::current_dir().unwrap();
            let version_file = cwd.join(".jules/.jo-version");
            fs::write(&version_file, "0.0.1\n").unwrap();

            let result = execute().expect("status should succeed");
            assert!(result.update_available);
            assert_eq!(result.workspace_version, Some("0.0.1".to_string()));
        });
    }
}
