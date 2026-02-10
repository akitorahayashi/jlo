//! Matrix workstreams command implementation.
//!
//! Exports enabled workstreams as a GitHub Actions matrix.

use serde::Serialize;

use crate::adapters::schedule_filesystem::load_schedule;
use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Options for matrix workstreams command.
#[derive(Debug, Clone, Default)]
pub struct MatrixWorkstreamsOptions {}

/// Output of matrix workstreams command.
#[derive(Debug, Clone, Serialize)]
pub struct MatrixWorkstreamsOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// GitHub Actions matrix object.
    pub matrix: GitHubMatrix,
    /// Number of workstreams in the matrix.
    pub count: usize,
    /// Whether any workstreams exist in the matrix.
    pub has_workstreams: bool,
}

/// GitHub Actions matrix structure.
#[derive(Debug, Clone, Serialize)]
pub struct GitHubMatrix {
    /// Matrix include entries.
    pub include: Vec<WorkstreamMatrixEntry>,
}

/// Single workstream matrix entry.
#[derive(Debug, Clone, Serialize)]
pub struct WorkstreamMatrixEntry {
    /// Workstream name.
    pub workstream: String,
}

/// Execute matrix workstreams command.
///
/// With the flat schedule model, this returns a single-entry matrix
/// if the root schedule is enabled.
pub fn execute(
    workspace: &impl WorkspaceStore,
    _options: MatrixWorkstreamsOptions,
) -> Result<MatrixWorkstreamsOutput, AppError> {
    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let mut include = Vec::new();

    match load_schedule(workspace) {
        Ok(schedule) if schedule.enabled => {
            include.push(WorkstreamMatrixEntry { workstream: "default".to_string() });
        }
        Ok(_) => {
            // Schedule exists but is disabled
        }
        Err(AppError::ScheduleConfigMissing(_)) => {
            // No schedule configured
        }
        Err(e) => return Err(e),
    }

    let count = include.len();
    let has_workstreams = !include.is_empty();

    Ok(MatrixWorkstreamsOutput {
        schema_version: 1,
        matrix: GitHubMatrix { include },
        count,
        has_workstreams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
    use serial_test::serial;

    use std::fs;
    use tempfile::tempdir;

    fn setup_workspace(root: &std::path::Path) {
        fs::create_dir_all(root.join(".jules")).unwrap();
        fs::write(root.join(".jules/version"), env!("CARGO_PKG_VERSION")).unwrap();
    }

    fn write_root_schedule(root: &std::path::Path, content: &str) {
        let dir = root.join(".jlo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scheduled.toml"), content).unwrap();
    }

    #[test]
    #[serial]
    fn returns_single_entry_when_schedule_enabled() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);
        let store = FilesystemWorkspaceStore::new(root.to_path_buf());

        write_root_schedule(
            root,
            r#"
version = 1
enabled = true
[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
"#,
        );

        let output = execute(&store, MatrixWorkstreamsOptions {}).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.count, 1);
        assert!(output.has_workstreams);
        assert_eq!(output.matrix.include[0].workstream, "default");
    }

    #[test]
    #[serial]
    fn empty_matrix_when_schedule_disabled() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);
        let store = FilesystemWorkspaceStore::new(root.to_path_buf());

        write_root_schedule(
            root,
            r#"
version = 1
enabled = false
[observers]
roles = []
"#,
        );

        let output = execute(&store, MatrixWorkstreamsOptions {}).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.count, 0);
        assert!(!output.has_workstreams);
        assert!(output.matrix.include.is_empty());
    }
}
