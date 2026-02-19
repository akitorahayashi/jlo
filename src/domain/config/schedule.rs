use crate::domain::RoleId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("Schedule config invalid: {0}")]
    ConfigInvalid(String),

    #[error("TOML format error: {0}")]
    Toml(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledRole {
    pub name: RoleId,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ScheduleLayer {
    #[serde(default)]
    pub roles: Vec<ScheduledRole>,
}

impl ScheduleLayer {
    pub fn enabled_roles(&self) -> Vec<RoleId> {
        self.roles.iter().filter(|r| r.enabled).map(|r| r.name.clone()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    #[serde(default)]
    pub observers: ScheduleLayer,
    #[serde(default)]
    pub innovators: Option<ScheduleLayer>,
}

impl Schedule {
    #[allow(dead_code)]
    pub fn parse_toml(content: &str) -> Result<Self, ScheduleError> {
        let schedule: Schedule =
            toml::from_str(content).map_err(|e| ScheduleError::Toml(e.to_string()))?;
        schedule.validate()?;
        Ok(schedule)
    }

    pub fn validate(&self) -> Result<(), ScheduleError> {
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
[observers]
roles = [
  { name = "taxonomy", enabled = true },
  { name = "qa", enabled = false },
]
"#;
        let schedule = Schedule::parse_toml(content).unwrap();
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
    fn missing_required_fields_use_defaults() {
        let content = r#"
"#;
        let schedule = Schedule::parse_toml(content).unwrap();
        assert!(schedule.observers.roles.is_empty());
        assert!(schedule.innovators.is_none());
    }

    #[test]
    fn invalid_role_ids_fail() {
        let content = r#"
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

    #[test]
    fn duplicate_role_ids_fail() {
        let content = r#"
[observers]
roles = [
  { name = "taxonomy", enabled = true },
  { name = "taxonomy", enabled = false },
]
"#;
        let err = Schedule::parse_toml(content).unwrap_err();
        assert!(matches!(err, ScheduleError::ConfigInvalid(_)));
        assert_eq!(
            err.to_string(),
            "Schedule config invalid: Duplicate role id 'taxonomy' in observers schedule"
        );
    }

    #[test]
    fn duplicate_role_ids_in_innovators_fail() {
        let content = r#"
[innovators]
roles = [
  { name = "taxonomy", enabled = true },
  { name = "taxonomy", enabled = false },
]
"#;
        let err = Schedule::parse_toml(content).unwrap_err();
        assert!(matches!(err, ScheduleError::ConfigInvalid(_)));
        assert_eq!(
            err.to_string(),
            "Schedule config invalid: Duplicate role id 'taxonomy' in innovators schedule"
        );
    }
}
