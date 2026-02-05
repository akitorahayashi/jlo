use std::collections::HashSet;

use crate::domain::{AppError, JULES_DIR, Layer, RoleId};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workstream_schedule_filesystem::load_schedule;

pub struct RoleSelectionInput<'a, W> {
    pub layer: Layer,
    pub workstream: &'a str,
    pub scheduled: bool,
    pub requested_roles: Option<&'a Vec<String>>,
    pub workspace: &'a W,
}

pub fn select_roles<W: WorkspaceStore>(
    input: RoleSelectionInput<'_, W>,
) -> Result<Vec<RoleId>, AppError> {
    ensure_workstream_exists(input.workspace, input.workstream)?;

    let roles = if input.scheduled {
        let schedule = load_schedule(input.workspace, input.workstream)?;
        if !schedule.enabled {
            return Err(AppError::Validation(format!(
                "Workstream '{}' is disabled in scheduled.toml",
                input.workstream
            )));
        }
        match input.layer {
            Layer::Observers => schedule.observers.enabled_roles(),
            Layer::Deciders => schedule.deciders.enabled_roles(),
            _ => {
                return Err(AppError::Validation(
                    "Scheduled mode is only supported for observers and deciders".into(),
                ));
            }
        }
    } else {
        input
            .requested_roles
            .filter(|roles| !roles.is_empty())
            .ok_or_else(|| {
                AppError::MissingArgument("Manual mode requires at least one --role value".into())
            })?
            .iter()
            .map(|s| RoleId::new(s))
            .collect::<Result<Vec<RoleId>, AppError>>()?
    };

    if roles.is_empty() && input.layer == Layer::Deciders && input.scheduled {
        return Ok(vec![]);
    }

    // Validate each role exists in the layer's roles directory
    let mut seen = HashSet::new();
    for role in &roles {
        if !seen.insert(role) {
            return Err(AppError::Validation(format!("Duplicate role '{}' specified", role)));
        }
        validate_role_exists(input.workspace, input.layer, role.as_str())?;
    }

    Ok(roles)
}

fn ensure_workstream_exists(
    workspace: &impl WorkspaceStore,
    workstream: &str,
) -> Result<(), AppError> {
    if !workspace.workstream_exists(workstream) {
        return Err(AppError::Validation(format!("Workstream '{}' not found", workstream)));
    }
    Ok(())
}

/// Validate that a role exists in the layer's roles directory.
///
/// For the new scaffold structure, roles are under:
/// `.jules/roles/<layer>/roles/<role>/role.yml`
fn validate_role_exists(
    workspace: &impl WorkspaceStore,
    layer: Layer,
    role: &str,
) -> Result<(), AppError> {
    let role_path = format!("{}/roles/{}/roles/{}/role.yml", JULES_DIR, layer.dir_name(), role);

    if !workspace.path_exists(&role_path) {
        return Err(AppError::RoleNotFound(format!(
            "{}/roles/{} (role.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;
    use tempfile::tempdir;

    fn write_schedule(root: &Path, ws: &str, content: &str) {
        let dir = root.join(".jules/workstreams").join(ws);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scheduled.toml"), content).unwrap();
    }

    fn write_role(root: &Path, layer: Layer, role: &str) {
        let dir = root.join(".jules/roles").join(layer.dir_name()).join("roles").join(role);
        fs::create_dir_all(&dir).unwrap();
        let role_yml = format!(
            "role: {role}\nlayer: {layer}\nprofile:\n  focus: test\n",
            role = role,
            layer = layer.dir_name()
        );
        fs::write(dir.join("role.yml"), role_yml).unwrap();
    }

    #[test]
    fn scheduled_roles_use_schedule() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let workspace = FilesystemWorkspaceStore::new(root.to_path_buf());
        workspace.create_structure(&[]).unwrap();

        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/observers")).unwrap();

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
        write_role(root, Layer::Observers, "taxonomy");

        let roles = select_roles(RoleSelectionInput {
            layer: Layer::Observers,
            workstream: "alpha",
            scheduled: true,
            requested_roles: None,
            workspace: &workspace,
        })
        .unwrap();

        assert_eq!(roles[0].as_str(), "taxonomy");
    }

    #[test]
    fn manual_roles_require_role_exists() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let workspace = FilesystemWorkspaceStore::new(root.to_path_buf());
        workspace.create_structure(&[]).unwrap();

        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/observers")).unwrap();
        write_role(root, Layer::Observers, "taxonomy");

        let roles = select_roles(RoleSelectionInput {
            layer: Layer::Observers,
            workstream: "alpha",
            scheduled: false,
            requested_roles: Some(&vec!["taxonomy".to_string()]),
            workspace: &workspace,
        })
        .unwrap();

        assert_eq!(roles[0].as_str(), "taxonomy");
    }

    #[test]
    fn missing_role_fails() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let workspace = FilesystemWorkspaceStore::new(root.to_path_buf());
        workspace.create_structure(&[]).unwrap();

        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/observers")).unwrap();

        let err = select_roles(RoleSelectionInput {
            layer: Layer::Observers,
            workstream: "alpha",
            scheduled: false,
            requested_roles: Some(&vec!["nonexistent".to_string()]),
            workspace: &workspace,
        })
        .unwrap_err();

        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn scheduled_deciders_can_be_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let workspace = FilesystemWorkspaceStore::new(root.to_path_buf());
        workspace.create_structure(&[]).unwrap();

        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/deciders")).unwrap();

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

        let roles = select_roles(RoleSelectionInput {
            layer: Layer::Deciders,
            workstream: "alpha",
            scheduled: true,
            requested_roles: None,
            workspace: &workspace,
        })
        .unwrap();

        assert!(roles.is_empty());
    }
}
