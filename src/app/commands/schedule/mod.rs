use serde::Serialize;

use crate::domain::{AppError, Layer, JULES_DIR};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workstream_schedule_filesystem::load_schedule;

#[derive(Debug, Clone)]
pub enum ScheduleExportScope {
    Workstreams,
    Roles,
}

#[derive(Debug, Clone)]
pub enum ScheduleExportFormat {
    GithubMatrix,
}

#[derive(Debug, Clone)]
pub struct ScheduleExportOptions {
    pub scope: ScheduleExportScope,
    pub layer: Option<Layer>,
    pub workstream: Option<String>,
    pub format: ScheduleExportFormat,
}

#[derive(Debug, Serialize)]
pub struct ScheduleMatrix {
    pub include: Vec<ScheduleMatrixEntry>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleMatrixEntry {
    pub workstream: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

pub fn export(
    workspace: &impl WorkspaceStore,
    options: ScheduleExportOptions,
) -> Result<ScheduleMatrix, AppError> {
    match options.format {
        ScheduleExportFormat::GithubMatrix => {}
    }

    match options.scope {
        ScheduleExportScope::Workstreams => export_workstreams(workspace),
        ScheduleExportScope::Roles => export_roles(workspace, options.layer, options.workstream),
    }
}

fn export_workstreams(workspace: &impl WorkspaceStore) -> Result<ScheduleMatrix, AppError> {
    let mut include = Vec::new();
    let workstreams_dir = format!("{}/workstreams", JULES_DIR);

    if !workspace.path_exists(&workstreams_dir) {
        return Ok(ScheduleMatrix { include });
    }

    for name in workspace.list_dirs(&workstreams_dir)? {
        if name.is_empty() {
            continue;
        }
        let schedule = load_schedule(workspace, &name)?;
        if schedule.enabled {
            include.push(ScheduleMatrixEntry { workstream: name, role: None });
        }
    }

    Ok(ScheduleMatrix { include })
}

fn export_roles(
    workspace: &impl WorkspaceStore,
    layer: Option<Layer>,
    workstream: Option<String>,
) -> Result<ScheduleMatrix, AppError> {
    let layer = layer.ok_or_else(|| {
        AppError::MissingArgument("Missing --layer for schedule export (roles scope)".into())
    })?;
    let workstream = workstream.ok_or_else(|| {
        AppError::MissingArgument("Missing --workstream for schedule export (roles scope)".into())
    })?;

    let schedule = load_schedule(workspace, &workstream)?;
    if !schedule.enabled {
        return Err(AppError::Validation(format!(
            "Workstream '{}' is disabled in scheduled.toml",
            workstream
        )));
    }

    let roles = match layer {
        Layer::Observers => schedule.observers.enabled_roles(),
        Layer::Deciders => schedule.deciders.enabled_roles(),
        _ => {
            return Err(AppError::Validation(
                "Schedule export roles scope only supports observers or deciders".into(),
            ));
        }
    };

    let include = roles
        .into_iter()
        .map(|role| ScheduleMatrixEntry { workstream: workstream.clone(), role: Some(role.into()) })
        .collect();

    Ok(ScheduleMatrix { include })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;
    use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

    fn write_schedule(root: &Path, ws: &str, content: &str) {
        let dir = root.join(".jules/workstreams").join(ws);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scheduled.toml"), content).unwrap();
    }

    #[test]
    fn export_workstreams_returns_enabled_only() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let workspace = FilesystemWorkspaceStore::new(root.to_path_buf());
        workspace.create_structure(&[]).unwrap();

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
roles = [
  { name = "taxonomy", enabled = true },
]
[deciders]
roles = []
"#,
        );

        let output = export(
            &workspace,
            ScheduleExportOptions {
                scope: ScheduleExportScope::Workstreams,
                layer: None,
                workstream: None,
                format: ScheduleExportFormat::GithubMatrix,
            },
        )
        .unwrap();

        assert_eq!(output.include.len(), 1);
        assert_eq!(output.include[0].workstream, "alpha");
        assert!(output.include[0].role.is_none());
    }

    #[test]
    fn export_roles_returns_layer_roles() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let workspace = FilesystemWorkspaceStore::new(root.to_path_buf());
        workspace.create_structure(&[]).unwrap();

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
]
[deciders]
roles = [
  { name = "triage_generic", enabled = true },
]
"#,
        );

        let output = export(
            &workspace,
            ScheduleExportOptions {
                scope: ScheduleExportScope::Roles,
                layer: Some(Layer::Observers),
                workstream: Some("alpha".to_string()),
                format: ScheduleExportFormat::GithubMatrix,
            },
        )
        .unwrap();

        let roles: Vec<String> =
            output.include.iter().map(|entry| entry.role.clone().unwrap()).collect();
        assert_eq!(roles, vec!["taxonomy", "qa"]);
    }
}
