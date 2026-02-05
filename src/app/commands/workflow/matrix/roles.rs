//! Matrix roles command implementation.
//!
//! Exports enabled roles for a multi-role layer as a GitHub Actions matrix.

use serde::{Deserialize, Serialize};

use crate::domain::{AppError, Layer};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::services::adapters::workstream_schedule_filesystem::load_schedule;

/// Options for matrix roles command.
#[derive(Debug, Clone)]
pub struct MatrixRolesOptions {
    /// Target layer (observers or deciders).
    pub layer: Layer,
    /// Workstreams JSON from `matrix workstreams` output.
    pub workstreams_json: WorkstreamsMatrix,
}

/// Input workstreams matrix (from matrix workstreams output).
#[derive(Debug, Clone, Deserialize)]
pub struct WorkstreamsMatrix {
    /// Matrix include entries.
    pub include: Vec<WorkstreamEntry>,
}

/// Single workstream entry from input matrix.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkstreamEntry {
    /// Workstream name.
    pub workstream: String,
}

/// Output of matrix roles command.
#[derive(Debug, Clone, Serialize)]
pub struct MatrixRolesOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// GitHub Actions matrix object.
    pub matrix: RolesMatrix,
    /// Number of roles in the matrix.
    pub count: usize,
    /// Whether any roles exist in the matrix.
    pub has_roles: bool,
}

/// GitHub Actions matrix structure for roles.
#[derive(Debug, Clone, Serialize)]
pub struct RolesMatrix {
    /// Matrix include entries.
    pub include: Vec<RoleMatrixEntry>,
}

/// Single role matrix entry.
#[derive(Debug, Clone, Serialize)]
pub struct RoleMatrixEntry {
    /// Workstream name.
    pub workstream: String,
    /// Role name.
    pub role: String,
}

/// Execute matrix roles command.
pub fn execute(options: MatrixRolesOptions) -> Result<MatrixRolesOutput, AppError> {
    // Validate layer is multi-role
    match options.layer {
        Layer::Observers | Layer::Deciders => {}
        _ => {
            return Err(AppError::Validation(
                "Matrix roles only supports observers or deciders layers".into(),
            ));
        }
    }

    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = workspace.jules_path();
    let mut include = Vec::new();

    for ws_entry in &options.workstreams_json.include {
        let schedule = load_schedule(&jules_path, &ws_entry.workstream)?;

        // Skip disabled workstreams (shouldn't happen if input is from matrix workstreams)
        if !schedule.enabled {
            continue;
        }

        let roles = match options.layer {
            Layer::Observers => schedule.observers.enabled_roles(),
            Layer::Deciders => schedule.deciders.enabled_roles(),
            _ => unreachable!(),
        };

        for role in roles {
            include.push(RoleMatrixEntry {
                workstream: ws_entry.workstream.clone(),
                role: role.into(),
            });
        }
    }

    // Ensure deterministic ordering: sort by workstream, then by role
    include.sort_by(|a, b| a.workstream.cmp(&b.workstream).then_with(|| a.role.cmp(&b.role)));

    let count = include.len();
    let has_roles = !include.is_empty();

    Ok(MatrixRolesOutput { schema_version: 1, matrix: RolesMatrix { include }, count, has_roles })
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
        fs::create_dir_all(root.join(".jules")).unwrap();
        fs::write(root.join(".jules/version"), env!("CARGO_PKG_VERSION")).unwrap();
    }

    #[test]
    fn returns_enabled_observer_roles() {
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
  { name = "qa", enabled = true },
  { name = "disabled_role", enabled = false },
]
[deciders]
roles = []
"#,
        );

        std::env::set_current_dir(root).unwrap();

        let workstreams_json =
            WorkstreamsMatrix { include: vec![WorkstreamEntry { workstream: "alpha".into() }] };

        let output =
            execute(MatrixRolesOptions { layer: Layer::Observers, workstreams_json }).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.count, 2);
        assert!(output.has_roles);

        let roles: Vec<(&str, &str)> = output
            .matrix
            .include
            .iter()
            .map(|e| (e.workstream.as_str(), e.role.as_str()))
            .collect();
        assert_eq!(roles, vec![("alpha", "qa"), ("alpha", "taxonomy")]);
    }

    #[test]
    fn rejects_invalid_layer() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        std::env::set_current_dir(root).unwrap();

        let result = execute(MatrixRolesOptions {
            layer: Layer::Planners,
            workstreams_json: WorkstreamsMatrix { include: vec![] },
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("only supports observers or deciders"));
    }
}
