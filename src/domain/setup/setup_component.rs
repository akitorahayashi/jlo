//! Setup component domain model.

use serde::Deserialize;

use crate::domain::setup::error::SetupError;
use crate::impl_validated_id;

/// A validated setup component identifier.
///
/// Guarantees:
/// - Non-empty
/// - Contains only alphanumeric characters, `-`, `_`, or `.`
/// - No path traversal components (/, \\, .., etc.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetupComponentId(String);

impl_validated_id!(SetupComponentId, true, SetupError, SetupError::InvalidComponentId);

/// Environment variable specification for a setup component.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EnvSpec {
    /// Variable name.
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Default value (if any).
    #[serde(default)]
    pub default: Option<String>,
    /// Whether this value must be stored as secret.
    #[serde(default)]
    pub secret: bool,
}

/// A setup component that can be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupComponent {
    /// Component name (unique identifier).
    pub name: SetupComponentId,
    /// Short summary of what this component provides.
    pub summary: String,
    /// Names of components this depends on.
    pub dependencies: Vec<SetupComponentId>,
    /// Environment variables this component uses.
    pub env: Vec<EnvSpec>,
    /// Installation script content.
    pub script_content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_alphanumeric_id() {
        assert!(SetupComponentId::new("rust").is_ok());
    }

    #[test]
    fn valid_id_with_dashes_and_dots() {
        assert!(SetupComponentId::new("node-v1.2").is_ok());
    }

    #[test]
    fn empty_id_is_invalid() {
        assert!(SetupComponentId::new("").is_err());
    }

    #[test]
    fn slash_in_id_is_invalid() {
        assert!(SetupComponentId::new("invalid/id").is_err());
    }

    #[test]
    fn dot_dot_is_invalid() {
        assert!(SetupComponentId::new("..").is_err());
    }

    #[test]
    fn display_impl() {
        let component = SetupComponentId::new("my.comp").unwrap();
        assert_eq!(format!("{}", component), "my.comp");
    }
}
