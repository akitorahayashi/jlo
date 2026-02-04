use crate::impl_validated_id;
use super::AppError;

/// A validated component identifier.
///
/// Guarantees:
/// - Non-empty
/// - Contains only alphanumeric characters, `-`, `_`, or `.`
/// - No path traversal components (/, \, .., etc.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(String);

impl_validated_id!(ComponentId, true, AppError::InvalidComponentId);

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

    #[test]
    fn display_impl() {
        let comp = ComponentId::new("my.comp").unwrap();
        assert_eq!(format!("{}", comp), "my.comp");
    }
}
