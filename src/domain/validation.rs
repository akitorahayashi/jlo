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
}
