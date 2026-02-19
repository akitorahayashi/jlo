use serde::{Deserialize, Serialize};

use crate::domain::AppError;

/// Header fields for a requirement.
///
/// This struct represents the YAML schema for requirement files. All fields are
/// retained for schema fidelity even if not directly consumed by current callers.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RequirementHeader {
    /// Label for the requirement (e.g., bugs, feats, refacts).
    #[serde(default)]
    pub label: String,
    /// Whether the requirement is ready for implementer execution.
    #[serde(default)]
    pub implementation_ready: bool,
}

impl RequirementHeader {
    /// Parse a requirement header from YAML content.
    pub fn parse(content: &str) -> Result<Self, AppError> {
        serde_yaml::from_str(content).map_err(|e| AppError::ParseError {
            what: "requirement".to_string(),
            details: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AppError;

    #[test]
    fn parse_requirement_header_success() {
        let header = RequirementHeader::parse("label: bugs\nimplementation_ready: true").unwrap();
        assert_eq!(header.label, "bugs");
        assert!(header.implementation_ready);
    }

    #[test]
    fn parse_requirement_header_default_values() {
        let header = RequirementHeader::parse("label: features").unwrap(); // Missing implementation_ready
        assert_eq!(header.label, "features");
        assert!(!header.implementation_ready); // Default should be false
    }

    #[test]
    fn parse_requirement_header_parse_error() {
        let result = RequirementHeader::parse("invalid: [ yaml");
        assert!(matches!(result, Err(AppError::ParseError { .. })));
    }
}
