/// Validates an identifier string.
///
/// Checks:
/// - Non-empty
/// - No path separators (/, \)
/// - Not "." or ".."
/// - Characters are alphanumeric, '-', '_', or (optionally) '.'
pub fn validate_identifier(id: &str, allow_dots: bool) -> bool {
    if id.is_empty() {
        return false;
    }
    if id.contains('/') || id.contains('\\') {
        return false;
    }
    if id == "." || id == ".." {
        return false;
    }
    id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || (allow_dots && c == '.'))
}

#[macro_export]
macro_rules! impl_validated_id {
    ($name:ident, $allow_dots:expr, $err_type:ty, $err_variant:expr) => {
        impl $name {
            /// Validate and create a new instance.
            pub fn new(id: &str) -> Result<Self, $err_type> {
                if $crate::domain::validation::validate_identifier(id, $allow_dots) {
                    Ok(Self(id.to_string()))
                } else {
                    Err($err_variant(id.to_string()))
                }
            }

            /// Return the inner string value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_identifiers() {
        assert!(validate_identifier("valid-id", false));
        assert!(validate_identifier("valid_id", false));
        assert!(validate_identifier("ValidId123", false));
    }

    #[test]
    fn valid_identifiers_with_dots() {
        assert!(validate_identifier("valid.id", true));
        assert!(!validate_identifier("valid.id", false));
    }

    #[test]
    fn invalid_identifiers() {
        assert!(!validate_identifier("", false));
        assert!(!validate_identifier("invalid/id", false));
        assert!(!validate_identifier("invalid\\id", false));
        assert!(!validate_identifier(".", false));
        assert!(!validate_identifier("..", false));
        assert!(!validate_identifier("has space", false));
        // Additional cases from removed tests
        assert!(!validate_identifier("../escape", false));
        assert!(!validate_identifier("../../..", false));
        assert!(!validate_identifier(".hidden", false));
        assert!(!validate_identifier("null\0byte", false));
    }
}
