use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Header fields for a requirement.
///
/// This struct represents the YAML schema for requirement files. All fields are
/// retained for schema fidelity even if not directly consumed by current callers.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RequirementHeader {
    /// Label for the requirement (e.g., bugs, feats, refacts).
    #[serde(default)]
    pub label: String,
    /// Whether the requirement requires deep analysis (planner) or implementation (implementer).
    #[serde(default)]
    pub requires_deep_analysis: bool,
}

impl RequirementHeader {
    /// Parse a requirement header from YAML content.
    pub fn parse(content: &str) -> Result<Self, AppError> {
        serde_yaml::from_str(content).map_err(|e| AppError::ParseError {
            what: "requirement".to_string(),
            details: e.to_string(),
        })
    }

    /// Read requirement header from a file in the workspace.
    pub fn read(store: &impl WorkspaceStore, path: &Path) -> Result<Self, AppError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| AppError::Validation(format!("Invalid path: {}", path.display())))?;
        let content = store.read_file(path_str)?;
        Self::parse(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AppError;
    use crate::testing::MockWorkspaceStore;

    #[test]
    fn read_requirement_header_success() {
        let store = MockWorkspaceStore::new()
            .with_file("req.yml", "label: bugs\nrequires_deep_analysis: true");

        let header = RequirementHeader::read(&store, Path::new("req.yml")).unwrap();
        assert_eq!(header.label, "bugs");
        assert!(header.requires_deep_analysis);
    }

    #[test]
    fn read_requirement_header_default_values() {
        let store = MockWorkspaceStore::new().with_file("req.yml", "label: features"); // Missing requires_deep_analysis

        let header = RequirementHeader::read(&store, Path::new("req.yml")).unwrap();
        assert_eq!(header.label, "features");
        assert!(!header.requires_deep_analysis); // Default should be false
    }

    #[test]
    fn read_requirement_header_file_not_found() {
        let store = MockWorkspaceStore::new();
        let result = RequirementHeader::read(&store, Path::new("nonexistent.yml"));
        assert!(result.is_err());
    }

    #[test]
    fn read_requirement_header_parse_error() {
        let store = MockWorkspaceStore::new().with_file("req.yml", "invalid: [ yaml");
        let result = RequirementHeader::read(&store, Path::new("req.yml"));
        assert!(matches!(result, Err(AppError::ParseError { .. })));
    }
}
