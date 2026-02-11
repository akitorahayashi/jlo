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
        let serialized = toml::to_string(&schedule).map_err(|err| AppError::ParseError {
            what: "scheduled.toml".to_string(),
            details: err.to_string(),
        })?;
        workspace.write_file(schedule_path, &format!("{}\n", serialized.trim_end()))?;
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
