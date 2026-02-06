//! Matrix workstreams command implementation.
//!
//! Exports enabled workstreams as a GitHub Actions matrix.

use serde::Serialize;

use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::adapters::workstream_schedule_filesystem::{list_subdirectories, load_schedule};
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
pub fn execute(_options: MatrixWorkstreamsOptions) -> Result<MatrixWorkstreamsOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = workspace.jules_path();
    let workstreams_dir = jules_path.join("workstreams");

    let mut include = Vec::new();

    for entry in list_subdirectories(&workspace, &workstreams_dir)? {
        let name =
            entry.file_name().map(|value| value.to_string_lossy().to_string()).unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        let schedule = load_schedule(&workspace, &name)?;
        if schedule.enabled {
            include.push(WorkstreamMatrixEntry { workstream: name });
        }
    }

    // Ensure deterministic ordering
    include.sort_by(|a, b| a.workstream.cmp(&b.workstream));

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
    use std::fs;
    use tempfile::tempdir;

    fn write_schedule(root: &std::path::Path, ws: &str, content: &str) {
        let dir = root.join(".jules/workstreams").join(ws);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scheduled.toml"), content).unwrap();
    }

    fn setup_workspace(root: &std::path::Path) {
        // Create minimal workspace structure
        fs::create_dir_all(root.join(".jules")).unwrap();
        fs::write(root.join(".jules/version"), env!("CARGO_PKG_VERSION")).unwrap();
    }

    #[test]
    fn returns_only_enabled_workstreams() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        write_schedule(
            root,
            "alpha",
            r#"
version = 1
enabled = true
[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
[deciders]
roles = []
"#,
        );

        write_schedule(
            root,
            "beta",
            r#"
version = 1
enabled = false
[observers]
roles = []
[deciders]
roles = []
"#,
        );

        write_schedule(
            root,
            "gamma",
            r#"
version = 1
enabled = true
[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
[deciders]
roles = []
"#,
        );

        std::env::set_current_dir(root).unwrap();

        let output = execute(MatrixWorkstreamsOptions {}).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.count, 2);
        assert!(output.has_workstreams);

        let names: Vec<&str> =
            output.matrix.include.iter().map(|e| e.workstream.as_str()).collect();
        // Should be sorted alphabetically
        assert_eq!(names, vec!["alpha", "gamma"]);
    }

    #[test]
    fn empty_matrix_when_no_enabled_workstreams() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        write_schedule(
            root,
            "disabled",
            r#"
version = 1
enabled = false
[observers]
roles = []
[deciders]
roles = []
"#,
        );

        std::env::set_current_dir(root).unwrap();

        let output = execute(MatrixWorkstreamsOptions {}).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.count, 0);
        assert!(!output.has_workstreams);
        assert!(output.matrix.include.is_empty());
    }
}
