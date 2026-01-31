use std::path::Path;

use serde::Serialize;

use crate::domain::{AppError, Layer};
use crate::services::{list_subdirectories, load_schedule};

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
    jules_path: &Path,
    options: ScheduleExportOptions,
) -> Result<ScheduleMatrix, AppError> {
    match options.format {
        ScheduleExportFormat::GithubMatrix => {}
    }

    match options.scope {
        ScheduleExportScope::Workstreams => export_workstreams(jules_path),
        ScheduleExportScope::Roles => export_roles(jules_path, options.layer, options.workstream),
    }
}

fn export_workstreams(jules_path: &Path) -> Result<ScheduleMatrix, AppError> {
    let mut include = Vec::new();
    let workstreams_dir = jules_path.join("workstreams");

    for entry in list_subdirectories(&workstreams_dir)? {
        let name =
            entry.file_name().map(|value| value.to_string_lossy().to_string()).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let schedule = load_schedule(jules_path, &name)?;
        if schedule.enabled {
            include.push(ScheduleMatrixEntry { workstream: name, role: None });
        }
    }

    Ok(ScheduleMatrix { include })
}

fn export_roles(
    jules_path: &Path,
    layer: Option<Layer>,
    workstream: Option<String>,
) -> Result<ScheduleMatrix, AppError> {
    let layer = layer.ok_or_else(|| {
        AppError::config_error("Missing --layer for schedule export (roles scope)")
    })?;
    let workstream = workstream.ok_or_else(|| {
        AppError::config_error("Missing --workstream for schedule export (roles scope)")
    })?;

    let schedule = load_schedule(jules_path, &workstream)?;
    if !schedule.enabled {
        return Err(AppError::config_error(format!(
            "Workstream '{}' is disabled in scheduled.toml",
            workstream
        )));
    }

    let roles = match layer {
        Layer::Observers => schedule.observers.roles,
        Layer::Deciders => schedule.deciders.roles,
        _ => {
            return Err(AppError::config_error(
                "Schedule export roles scope only supports observers or deciders",
            ));
        }
    };

    let include = roles
        .into_iter()
        .map(|role| ScheduleMatrixEntry { workstream: workstream.clone(), role: Some(role) })
        .collect();

    Ok(ScheduleMatrix { include })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_schedule(root: &Path, ws: &str, content: &str) {
        let dir = root.join(".jules/workstreams").join(ws);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scheduled.toml"), content).unwrap();
    }

    #[test]
    fn export_workstreams_returns_enabled_only() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let jules_path = root.join(".jules");

        write_schedule(
            root,
            "alpha",
            r#"
version = 1
enabled = true
[observers]
roles = ["taxonomy"]
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
roles = ["taxonomy"]
[deciders]
roles = []
"#,
        );

        let output = export(
            &jules_path,
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
        let jules_path = root.join(".jules");

        write_schedule(
            root,
            "alpha",
            r#"
version = 1
enabled = true
[observers]
roles = ["taxonomy", "qa"]
[deciders]
roles = ["triage_generic"]
"#,
        );

        let output = export(
            &jules_path,
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
