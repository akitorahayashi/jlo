use crate::domain::configuration::schedule::{ScheduleLayer, ScheduledRole};
use crate::domain::{AppError, Layer, RoleId, Schedule};
use crate::ports::WorkspaceStore;

pub fn ensure_role_scheduled<W: WorkspaceStore>(
    workspace: &W,
    layer: Layer,
    role: &RoleId,
) -> Result<bool, AppError> {
    if layer.is_single_role() {
        return Err(AppError::Validation(format!(
            "Layer '{}' does not support scheduling",
            layer.dir_name()
        )));
    }

    let schedule_path = ".jlo/scheduled.toml";
    let content = workspace.read_file(schedule_path)?;
    let mut schedule = Schedule::parse_toml(&content)?;

    let updated = match layer {
        Layer::Observers => insert_role(&mut schedule.observers, role),
        Layer::Innovators => {
            let target = schedule.innovators.get_or_insert_with(|| ScheduleLayer { roles: vec![] });
            insert_role(target, role)
        }
        Layer::Narrator | Layer::Decider | Layer::Planner | Layer::Implementer => false,
    };

    if updated {
        let serialized = render_schedule_toml(&schedule);
        workspace.write_file(schedule_path, &serialized)?;
    }

    Ok(updated)
}

fn insert_role(layer: &mut ScheduleLayer, role: &RoleId) -> bool {
    if layer.roles.iter().any(|entry| entry.name == *role) {
        return false;
    }

    layer.roles.push(ScheduledRole { name: role.clone(), enabled: true });
    true
}

fn render_schedule_toml(schedule: &Schedule) -> String {
    let mut lines = vec![
        format!("version = {}", schedule.version),
        format!("enabled = {}", schedule.enabled),
        String::new(),
    ];

    append_layer_toml(&mut lines, "observers", &schedule.observers);

    if let Some(innovators) = &schedule.innovators {
        lines.push(String::new());
        append_layer_toml(&mut lines, "innovators", innovators);
    }

    lines.join("\n") + "\n"
}

fn append_layer_toml(lines: &mut Vec<String>, layer_name: &str, layer: &ScheduleLayer) {
    lines.push(format!("[{}]", layer_name));
    if layer.roles.is_empty() {
        lines.push("roles = []".to_string());
        return;
    }

    lines.push("roles = [".to_string());
    for role in &layer.roles {
        lines.push(format!(
            "  {{ name = \"{}\", enabled = {} }},",
            role.name.as_str(),
            role.enabled
        ));
    }
    lines.push("]".to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockWorkspaceStore;

    #[test]
    fn ensure_role_scheduled_keeps_scaffold_style_for_observers() {
        let workspace = MockWorkspaceStore::new().with_file(
            ".jlo/scheduled.toml",
            r#"version = 1
enabled = true

[observers]
roles = [
  { name = "consistency", enabled = true },
]

[innovators]
roles = [
  { name = "recruiter", enabled = false },
]
"#,
        );

        let updated = ensure_role_scheduled(
            &workspace,
            Layer::Observers,
            &RoleId::new("librarian").expect("valid role id"),
        )
        .expect("schedule update should succeed");
        assert!(updated);

        let actual =
            workspace.read_file(".jlo/scheduled.toml").expect("written schedule should exist");
        let expected = r#"version = 1
enabled = true

[observers]
roles = [
  { name = "consistency", enabled = true },
  { name = "librarian", enabled = true },
]

[innovators]
roles = [
  { name = "recruiter", enabled = false },
]
"#;

        assert_eq!(actual, expected);
        assert!(!actual.contains("[[observers.roles]]"));
        assert!(!actual.contains("[[innovators.roles]]"));
    }

    #[test]
    fn ensure_role_scheduled_adds_innovators_section_in_scaffold_style() {
        let workspace = MockWorkspaceStore::new().with_file(
            ".jlo/scheduled.toml",
            r#"version = 1
enabled = true

[observers]
roles = [
  { name = "consistency", enabled = true },
]
"#,
        );

        let updated = ensure_role_scheduled(
            &workspace,
            Layer::Innovators,
            &RoleId::new("librarian").expect("valid role id"),
        )
        .expect("schedule update should succeed");
        assert!(updated);

        let actual =
            workspace.read_file(".jlo/scheduled.toml").expect("written schedule should exist");
        let expected = r#"version = 1
enabled = true

[observers]
roles = [
  { name = "consistency", enabled = true },
]

[innovators]
roles = [
  { name = "librarian", enabled = true },
]
"#;
        assert_eq!(actual, expected);
    }
}
