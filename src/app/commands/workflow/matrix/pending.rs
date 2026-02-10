//! Matrix pending command implementation.
//!
//! Checks the flat exchange directory for pending events and exports a
//! single-entry GitHub Actions matrix when pending events exist.

use serde::Serialize;
use std::fs;

use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Options for matrix pending command.
#[derive(Debug, Clone)]
pub struct MatrixPendingOptions {
    /// Mock mode - always report pending events.
    pub mock: bool,
}

/// Output of matrix pending command.
#[derive(Debug, Clone, Serialize)]
pub struct MatrixPendingOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Whether pending events exist.
    pub has_pending: bool,
}

/// Execute matrix pending command.
pub fn execute(
    workspace: &impl WorkspaceStore,
    options: MatrixPendingOptions,
) -> Result<MatrixPendingOutput, AppError> {
    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    if options.mock {
        return Ok(MatrixPendingOutput { schema_version: 1, has_pending: true });
    }

    let jules_path = workspace.jules_path();
    let pending_dir = jules_path.join("exchange/events/pending");

    let has_pending = pending_dir.exists() && has_yml_files(&pending_dir)?;

    Ok(MatrixPendingOutput { schema_version: 1, has_pending })
}

/// Check if a directory contains any .yml files.
fn has_yml_files(dir: &std::path::Path) -> Result<bool, AppError> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "yml").unwrap_or(false) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
    use std::fs;
    use tempfile::tempdir;

    fn setup_workspace(root: &std::path::Path) {
        fs::create_dir_all(root.join(".jules")).unwrap();
        fs::write(root.join(".jules/version"), env!("CARGO_PKG_VERSION")).unwrap();
    }

    #[test]
    fn returns_has_pending_when_events_exist() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);
        let store = FilesystemWorkspaceStore::new(root.to_path_buf());

        let pending_dir = root.join(".jules/exchange/events/pending");
        fs::create_dir_all(&pending_dir).unwrap();
        fs::write(pending_dir.join("event1.yml"), "id: abc123\n").unwrap();

        let output = execute(&store, MatrixPendingOptions { mock: false }).unwrap();

        assert_eq!(output.schema_version, 1);
        assert!(output.has_pending);
    }

    #[test]
    fn returns_no_pending_when_dir_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);
        let store = FilesystemWorkspaceStore::new(root.to_path_buf());

        let pending_dir = root.join(".jules/exchange/events/pending");
        fs::create_dir_all(&pending_dir).unwrap();

        let output = execute(&store, MatrixPendingOptions { mock: false }).unwrap();

        assert_eq!(output.schema_version, 1);
        assert!(!output.has_pending);
    }

    #[test]
    fn mock_mode_always_reports_pending() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);
        let store = FilesystemWorkspaceStore::new(root.to_path_buf());

        let output = execute(&store, MatrixPendingOptions { mock: true }).unwrap();

        assert!(output.has_pending);
    }
}
