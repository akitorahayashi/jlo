use serde::Deserialize;

use crate::domain::RoleId;

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("Schedule config invalid: {0}")]
    ConfigInvalid(String),

    #[error("TOML format error: {0}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkstreamSchedule {
    pub version: u32,
    pub enabled: bool,
    pub observers: ScheduleLayer,
    pub deciders: ScheduleLayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleLayer {
    pub roles: Vec<ScheduledRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledRole {
    pub name: String,
    pub enabled: bool,
}

impl ScheduleLayer {
    pub fn enabled_roles(&self) -> Vec<String> {
        self.roles.iter().filter(|r| r.enabled).map(|r| r.name.clone()).collect()
    }
}

impl WorkstreamSchedule {
    pub fn parse_toml(content: &str) -> Result<Self, ScheduleError> {
        let dto: dto::ScheduleDto = toml::from_str(content)?;
        let schedule: WorkstreamSchedule = dto.try_into().map_err(ScheduleError::ConfigInvalid)?;
        schedule.validate()?;
        Ok(schedule)
    }

    fn validate(&self) -> Result<(), ScheduleError> {
        if self.version != 1 {
            return Err(ScheduleError::ConfigInvalid(format!(
                "Unsupported scheduled.toml version: {} (expected 1)",
                self.version
            )));
        }

        if self.enabled && self.observers.roles.is_empty() {
            return Err(ScheduleError::ConfigInvalid(
                "scheduled.toml enabled=true requires at least one observer role".into(),
            ));
        }

        Self::validate_roles("observers", &self.observers)?;
        Self::validate_roles("deciders", &self.deciders)?;

        Ok(())
    }

    fn validate_roles(layer: &str, schedule_layer: &ScheduleLayer) -> Result<(), ScheduleError> {
        let mut seen = std::collections::HashSet::new();
        for role in &schedule_layer.roles {
            RoleId::new(&role.name).map_err(|_| {
                ScheduleError::ConfigInvalid(format!(
                    "Invalid role id '{}' in {} schedule",
                    role.name, layer
                ))
            })?;
            if !seen.insert(&role.name) {
                return Err(ScheduleError::ConfigInvalid(format!(
                    "Duplicate role id '{}' in {} schedule",
                    role.name, layer
                )));
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
        pub roles: Option<Vec<RoleEntryDto>>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RoleEntryDto {
        pub name: String,
        pub enabled: bool,
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
                .ok_or_else(|| "scheduled.toml missing observers.roles".to_string())?
                .into_iter()
                .map(|r| ScheduledRole { name: r.name, enabled: r.enabled })
                .collect();
            let deciders_roles = deciders
                .roles
                .ok_or_else(|| "scheduled.toml missing deciders.roles".to_string())?
                .into_iter()
                .map(|r| ScheduledRole { name: r.name, enabled: r.enabled })
                .collect();

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
roles = [
  { name = "taxonomy", enabled = true },
  { name = "qa", enabled = false },
]

[deciders]
roles = [
  { name = "triage_generic", enabled = true },
]
"#;
        let schedule = WorkstreamSchedule::parse_toml(content).unwrap();
        assert_eq!(schedule.version, 1);
        assert!(schedule.enabled);
        assert_eq!(schedule.observers.roles.len(), 2);
        assert_eq!(schedule.observers.roles[0].name, "taxonomy");
        assert!(schedule.observers.roles[0].enabled);
        assert_eq!(schedule.observers.roles[1].name, "qa");
        assert!(!schedule.observers.roles[1].enabled);
        assert_eq!(schedule.observers.enabled_roles(), vec!["taxonomy"]);
        assert_eq!(schedule.deciders.enabled_roles(), vec!["triage_generic"]);
    }

    #[test]
    fn missing_required_fields_fail() {
        let content = r#"
version = 1
enabled = true
"#;
        let err = WorkstreamSchedule::parse_toml(content).unwrap_err();
        assert!(err.to_string().contains("missing [observers]"));
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
        assert!(err.to_string().contains("requires at least one observer role"));
    }

    #[test]
    fn invalid_role_ids_fail() {
        let content = r#"
version = 1
enabled = false

[observers]
roles = [
  { name = "bad role", enabled = true },
]

[deciders]
roles = []
"#;
        let err = WorkstreamSchedule::parse_toml(content).unwrap_err();
        assert!(err.to_string().contains("Invalid role id"));
    }
}
