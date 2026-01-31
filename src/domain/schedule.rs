use serde::Deserialize;

use crate::domain::RoleId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkstreamSchedule {
    pub version: u32,
    pub enabled: bool,
    pub observers: ScheduleLayer,
    pub deciders: ScheduleLayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleLayer {
    pub roles: Vec<String>,
}

impl WorkstreamSchedule {
    pub fn parse_toml(content: &str) -> Result<Self, String> {
        let dto: dto::ScheduleDto = toml::from_str(content).map_err(|e| e.to_string())?;
        let schedule: WorkstreamSchedule = dto.try_into()?;
        schedule.validate()?;
        Ok(schedule)
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!(
                "Unsupported scheduled.toml version: {} (expected 1)",
                self.version
            ));
        }

        if self.enabled && self.observers.roles.is_empty() {
            return Err("scheduled.toml enabled=true requires at least one observer role".into());
        }

        Self::validate_roles("observers", &self.observers.roles)?;
        Self::validate_roles("deciders", &self.deciders.roles)?;

        Ok(())
    }

    fn validate_roles(layer: &str, roles: &[String]) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for role in roles {
            RoleId::new(role)
                .map_err(|_| format!("Invalid role id '{}' in {} schedule", role, layer))?;
            if !seen.insert(role) {
                return Err(format!("Duplicate role id '{}' in {} schedule", role, layer));
            }
        }
        Ok(())
    }
}

mod dto {
    use super::*;

    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ScheduleDto {
        pub version: Option<u32>,
        pub enabled: Option<bool>,
        pub observers: Option<ScheduleLayerDto>,
        pub deciders: Option<ScheduleLayerDto>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ScheduleLayerDto {
        pub roles: Option<Vec<String>>,
    }

    impl TryFrom<ScheduleDto> for WorkstreamSchedule {
        type Error = String;

        fn try_from(dto: ScheduleDto) -> Result<Self, Self::Error> {
            let version =
                dto.version.ok_or_else(|| "scheduled.toml missing version".to_string())?;
            let enabled =
                dto.enabled.ok_or_else(|| "scheduled.toml missing enabled".to_string())?;

            let observers =
                dto.observers.ok_or_else(|| "scheduled.toml missing [observers]".to_string())?;
            let deciders =
                dto.deciders.ok_or_else(|| "scheduled.toml missing [deciders]".to_string())?;

            let observers_roles = observers
                .roles
                .ok_or_else(|| "scheduled.toml missing observers.roles".to_string())?;
            let deciders_roles = deciders
                .roles
                .ok_or_else(|| "scheduled.toml missing deciders.roles".to_string())?;

            Ok(WorkstreamSchedule {
                version,
                enabled,
                observers: ScheduleLayer { roles: observers_roles },
                deciders: ScheduleLayer { roles: deciders_roles },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_schedule() {
        let content = r#"
version = 1
enabled = true

[observers]
roles = ["taxonomy", "qa"]

[deciders]
roles = ["triage_generic"]
"#;
        let schedule = WorkstreamSchedule::parse_toml(content).unwrap();
        assert_eq!(schedule.version, 1);
        assert!(schedule.enabled);
        assert_eq!(schedule.observers.roles, vec!["taxonomy", "qa"]);
        assert_eq!(schedule.deciders.roles, vec!["triage_generic"]);
    }

    #[test]
    fn missing_required_fields_fail() {
        let content = r#"
version = 1
enabled = true
"#;
        let err = WorkstreamSchedule::parse_toml(content).unwrap_err();
        assert!(err.contains("missing [observers]"));
    }

    #[test]
    fn enabled_requires_observer_roles() {
        let content = r#"
version = 1
enabled = true

[observers]
roles = []

[deciders]
roles = []
"#;
        let err = WorkstreamSchedule::parse_toml(content).unwrap_err();
        assert!(err.contains("requires at least one observer role"));
    }

    #[test]
    fn invalid_role_ids_fail() {
        let content = r#"
version = 1
enabled = false

[observers]
roles = ["bad role"]

[deciders]
roles = []
"#;
        let err = WorkstreamSchedule::parse_toml(content).unwrap_err();
        assert!(err.contains("Invalid role id"));
    }
}
