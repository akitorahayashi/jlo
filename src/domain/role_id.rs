use serde::{Deserialize, Deserializer};
use crate::impl_validated_id;
use super::AppError;

/// A validated role identifier.
///
/// Guarantees:
/// - Non-empty
/// - Contains only alphanumeric characters, `-`, or `_`
/// - No path traversal components (/, \, ., ..)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoleId(String);

impl_validated_id!(RoleId, false, AppError::InvalidRoleId);

impl From<RoleId> for String {
    fn from(val: RoleId) -> Self {
        val.0
    }
}

impl<'de> Deserialize<'de> for RoleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        RoleId::new(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_alphanumeric_id() {
        assert!(RoleId::new("taxonomy").is_ok());
    }

    #[test]
    fn valid_id_with_dashes() {
        assert!(RoleId::new("my-role-1").is_ok());
    }

    #[test]
    fn valid_id_with_underscore() {
        assert!(RoleId::new("data_arch").is_ok());
    }

    #[test]
    fn empty_id_is_invalid() {
        assert!(RoleId::new("").is_err());
    }

    #[test]
    fn slash_in_id_is_invalid() {
        assert!(RoleId::new("invalid/id").is_err());
    }

    #[test]
    fn dot_dot_is_invalid() {
        assert!(RoleId::new("..").is_err());
    }

    #[test]
    fn space_in_id_is_invalid() {
        assert!(RoleId::new("has space").is_err());
    }

    #[test]
    fn display_impl() {
        let role = RoleId::new("test-role").unwrap();
        assert_eq!(format!("{}", role), "test-role");
    }
}
