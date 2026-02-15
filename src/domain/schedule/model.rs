use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::domain::RoleId;
use super::error::ScheduleError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledRole {
    pub name: RoleId,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleLayer {
    pub roles: Vec<ScheduledRole>,
}

impl ScheduleLayer {
    pub fn enabled_roles(&self) -> Vec<RoleId> {
        self.roles.iter().filter(|r| r.enabled).map(|r| r.name.clone()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    pub version: u32,
    pub enabled: bool,
    pub observers: ScheduleLayer,
    #[serde(default)]
    pub innovators: Option<ScheduleLayer>,
}

impl Schedule {
    pub fn parse_toml(content: &str) -> Result<Self, ScheduleError> {
        let schedule: Schedule = toml::from_str(content)?;
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
        if let Some(ref innovators) = self.innovators {
            Self::validate_roles("innovators", innovators)?;
        }

        Ok(())
    }

    fn validate_roles(layer: &str, schedule_layer: &ScheduleLayer) -> Result<(), ScheduleError> {
        let mut seen = HashSet::new();
        for role in &schedule_layer.roles {
            if !seen.insert(role.name.clone()) {
                return Err(ScheduleError::ConfigInvalid(format!(
                    "Duplicate role id '{}' in {} schedule",
                    role.name.as_str(),
                    layer
                )));
            }
        }
        Ok(())
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
"#;
        let schedule = Schedule::parse_toml(content).unwrap();
        assert_eq!(schedule.version, 1);
        assert!(schedule.enabled);
        assert_eq!(schedule.observers.roles.len(), 2);
        assert_eq!(schedule.observers.roles[0].name.as_str(), "taxonomy");
        assert!(schedule.observers.roles[0].enabled);
        assert_eq!(schedule.observers.roles[1].name.as_str(), "qa");
        assert!(!schedule.observers.roles[1].enabled);

        let obs_roles: Vec<String> =
            schedule.observers.enabled_roles().into_iter().map(|r| r.into()).collect();
        assert_eq!(obs_roles, vec!["taxonomy"]);
    }

    #[test]
    fn missing_required_fields_fail() {
        let content = r#"
version = 1
enabled = true
"#;
        let err = Schedule::parse_toml(content).unwrap_err();
        // toml error will complain about missing fields
        assert!(matches!(err, ScheduleError::Toml(_)));
    }

    #[test]
    fn enabled_requires_observer_roles() {
        let content = r#"
version = 1
enabled = true

[observers]
roles = []
"#;
        let err = Schedule::parse_toml(content).unwrap_err();
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
"#;
        let err = Schedule::parse_toml(content).unwrap_err();
        // This will be a Toml error because deserialization of RoleId fails
        assert!(matches!(err, ScheduleError::Toml(_)));
        assert!(err.to_string().contains("Invalid role identifier"));
    }
}
