use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_yaml::Value;

use crate::domain::{AppError, Layer};
use crate::services::load_schedule;

pub struct RoleSelectionInput<'a> {
    pub jules_path: &'a Path,
    pub layer: Layer,
    pub workstream: &'a str,
    pub scheduled: bool,
    pub requested_roles: Option<&'a Vec<String>>,
}

pub fn select_roles(input: RoleSelectionInput<'_>) -> Result<Vec<String>, AppError> {
    ensure_workstream_exists(input.jules_path, input.workstream)?;

    let roles = if input.scheduled {
        let schedule = load_schedule(input.jules_path, input.workstream)?;
        if !schedule.enabled {
            return Err(AppError::config_error(format!(
                "Workstream '{}' is disabled in scheduled.toml",
                input.workstream
            )));
        }
        match input.layer {
            Layer::Observers => schedule.observers.roles,
            Layer::Deciders => schedule.deciders.roles,
            _ => {
                return Err(AppError::config_error(
                    "Scheduled mode is only supported for observers and deciders",
                ));
            }
        }
    } else {
        let requested = input.requested_roles.ok_or_else(|| {
            AppError::config_error("Manual mode requires at least one --role value")
        })?;
        if requested.is_empty() {
            return Err(AppError::config_error("Manual mode requires at least one --role value"));
        }
        requested.clone()
    };

    if roles.is_empty() && input.layer == Layer::Deciders && input.scheduled {
        return Ok(vec![]);
    }

    let mut seen = HashSet::new();
    for role in &roles {
        if !seen.insert(role) {
            return Err(AppError::config_error(format!("Duplicate role '{}' specified", role)));
        }
        let role_workstream = read_role_workstream(input.jules_path, input.layer, role)?;
        if role_workstream != input.workstream {
            return Err(AppError::config_error(format!(
                "Role '{}' targets workstream '{}' but '{}' was requested",
                role, role_workstream, input.workstream
            )));
        }
    }

    Ok(roles)
}

fn ensure_workstream_exists(jules_path: &Path, workstream: &str) -> Result<(), AppError> {
    let path = jules_path.join("workstreams").join(workstream);
    if !path.exists() {
        return Err(AppError::config_error(format!("Workstream '{}' not found", workstream)));
    }
    Ok(())
}

fn read_role_workstream(jules_path: &Path, layer: Layer, role: &str) -> Result<String, AppError> {
    let role_dir = jules_path.join("roles").join(layer.dir_name()).join(role);
    let prompt_path = role_dir.join("prompt.yml");
    if !prompt_path.exists() {
        return Err(AppError::RoleNotFound(format!(
            "{}/{} (prompt.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    let content = fs::read_to_string(&prompt_path)?;
    let value: Value = serde_yaml::from_str(&content).map_err(|err| {
        AppError::config_error(format!("Failed to parse {}: {}", prompt_path.display(), err))
    })?;
    let map = match value {
        Value::Mapping(map) => map,
        _ => {
            return Err(AppError::config_error(format!(
                "Prompt file {} must contain a mapping",
                prompt_path.display()
            )));
        }
    };

    let workstream = map
        .get(Value::String("workstream".to_string()))
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            AppError::config_error(format!(
                "Prompt file {} missing workstream field",
                prompt_path.display()
            ))
        })?;

    if workstream.trim().is_empty() {
        return Err(AppError::config_error(format!(
            "Prompt file {} has empty workstream field",
            prompt_path.display()
        )));
    }

    Ok(workstream.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_schedule(root: &Path, ws: &str, content: &str) {
        let dir = root.join(".jules/workstreams").join(ws);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scheduled.toml"), content).unwrap();
    }

    fn write_prompt(root: &Path, layer: Layer, role: &str, workstream: &str) {
        let dir = root.join(".jules/roles").join(layer.dir_name()).join(role);
        fs::create_dir_all(&dir).unwrap();
        let prompt = format!(
            "role: {role}\nlayer: {layer}\nworkstream: {workstream}\n",
            role = role,
            layer = layer.dir_name(),
            workstream = workstream
        );
        let path = dir.join("prompt.yml");
        fs::write(&path, prompt).unwrap();
    }

    #[test]
    fn scheduled_roles_use_schedule() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/observers")).unwrap();

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
        write_prompt(root, Layer::Observers, "taxonomy", "alpha");

        let roles = select_roles(RoleSelectionInput {
            jules_path: &root.join(".jules"),
            layer: Layer::Observers,
            workstream: "alpha",
            scheduled: true,
            requested_roles: None,
        })
        .unwrap();

        assert_eq!(roles, vec!["taxonomy"]);
    }

    #[test]
    fn manual_roles_require_match() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/observers")).unwrap();
        write_prompt(root, Layer::Observers, "taxonomy", "alpha");

        let roles = select_roles(RoleSelectionInput {
            jules_path: &root.join(".jules"),
            layer: Layer::Observers,
            workstream: "alpha",
            scheduled: false,
            requested_roles: Some(&vec!["taxonomy".to_string()]),
        })
        .unwrap();

        assert_eq!(roles, vec!["taxonomy"]);
    }

    #[test]
    fn manual_mismatch_fails() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/observers")).unwrap();
        write_prompt(root, Layer::Observers, "taxonomy", "other");

        let err = select_roles(RoleSelectionInput {
            jules_path: &root.join(".jules"),
            layer: Layer::Observers,
            workstream: "alpha",
            scheduled: false,
            requested_roles: Some(&vec!["taxonomy".to_string()]),
        })
        .unwrap_err();

        assert!(err.to_string().contains("targets workstream"));
    }

    #[test]
    fn scheduled_deciders_can_be_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".jules/workstreams/alpha")).unwrap();
        fs::create_dir_all(root.join(".jules/roles/deciders")).unwrap();

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

        let roles = select_roles(RoleSelectionInput {
            jules_path: &root.join(".jules"),
            layer: Layer::Deciders,
            workstream: "alpha",
            scheduled: true,
            requested_roles: None,
        })
        .unwrap();

        assert!(roles.is_empty());
    }
}
