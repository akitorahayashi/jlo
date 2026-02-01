use super::AppError;

/// A validated component identifier.
///
/// Guarantees:
/// - Non-empty
/// - Contains only alphanumeric characters, `-`, `_`, or `.`
/// - No path traversal components (/, \, .., etc.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(String);

impl ComponentId {
    /// Validate and create a new `ComponentId`.
    pub fn new(id: &str) -> Result<Self, AppError> {
        if Self::is_valid(id) {
            Ok(Self(id.to_string()))
        } else {
            Err(AppError::InvalidComponentId(id.to_string()))
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
            && id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    }
}

impl AsRef<str> for ComponentId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_alphanumeric_id() {
        assert!(ComponentId::new("rust").is_ok());
    }

    #[test]
    fn valid_id_with_dashes_and_dots() {
        assert!(ComponentId::new("node-v1.2").is_ok());
    }

    #[test]
    fn empty_id_is_invalid() {
        assert!(ComponentId::new("").is_err());
    }

    #[test]
    fn slash_in_id_is_invalid() {
        assert!(ComponentId::new("invalid/id").is_err());
    }

    #[test]
    fn dot_dot_is_invalid() {
        assert!(ComponentId::new("..").is_err());
    }
}
