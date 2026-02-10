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

/// Validates a path component for safe filesystem operations.
///
/// This is stricter than validate_identifier - used for user-provided path components
/// like exchange labels or states to prevent path traversal attacks.
///
/// Checks:
/// - Non-empty
/// - No path separators (/, \)
/// - Not "." or ".."
/// - Does not start with '.' (hidden files)
/// - No null bytes
/// - Characters are alphanumeric, '-', or '_' only (no dots)
pub fn validate_safe_path_component(component: &str) -> bool {
    if component.is_empty() || component.starts_with('.') {
        return false;
    }
    if component.contains('/') || component.contains('\\') || component.contains('\0') {
        return false;
    }
    if component == "." || component == ".." {
        return false;
    }
    component.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[macro_export]
macro_rules! impl_validated_id {
    ($name:ident, $allow_dots:expr, $err_variant:path) => {
        impl $name {
            /// Validate and create a new instance.
            pub fn new(id: &str) -> Result<Self, $crate::domain::AppError> {
                if $crate::domain::identifiers::validation::validate_identifier(id, $allow_dots) {
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
    }

    #[test]
    fn safe_path_component_valid() {
        assert!(validate_safe_path_component("valid-name"));
        assert!(validate_safe_path_component("valid_name"));
        assert!(validate_safe_path_component("ValidName123"));
    }

    #[test]
    fn safe_path_component_invalid() {
        assert!(!validate_safe_path_component(""));
        assert!(!validate_safe_path_component("../escape"));
        assert!(!validate_safe_path_component("../../.."));
        assert!(!validate_safe_path_component(".hidden"));
        assert!(!validate_safe_path_component("has/slash"));
        assert!(!validate_safe_path_component("has\\backslash"));
        assert!(!validate_safe_path_component("."));
        assert!(!validate_safe_path_component(".."));
        assert!(!validate_safe_path_component("has.dot"));
        assert!(!validate_safe_path_component("null\0byte"));
    }
}
