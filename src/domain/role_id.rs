use super::AppError;

/// A validated role identifier.
///
/// Guarantees:
/// - Non-empty
/// - Contains only alphanumeric characters, `-`, or `_`
/// - No path traversal components (/, \, ., ..)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleId(String);

impl RoleId {
    /// Validate and create a new `RoleId`.
    pub fn new(id: &str) -> Result<Self, AppError> {
        if Self::is_valid(id) {
            Ok(Self(id.to_string()))
        } else {
            Err(AppError::InvalidRoleId(id.to_string()))
        }
    }

    /// Return the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(id: &str) -> bool {
        !id.is_empty()
            && !id.contains('/')
            && !id.contains('\\')
            && id != "."
            && id != ".."
            && id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }
}

impl AsRef<str> for RoleId {
    fn as_ref(&self) -> &str {
        self.as_str()
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
}
