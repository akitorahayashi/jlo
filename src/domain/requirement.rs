use serde::Deserialize;
use std::path::Path;

use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Header fields for a requirement.
#[derive(Debug, Clone, Deserialize)]
pub struct RequirementHeader {
    /// Label for the requirement (e.g., bugs, feats, refacts).
    #[serde(default)]
    pub label: String,
    /// Whether the requirement requires deep analysis (planner) or implementation (implementer).
    pub requires_deep_analysis: bool,
}

impl RequirementHeader {
    /// Read requirement header from a file in the workspace.
    pub fn read(store: &impl WorkspaceStore, path: &Path) -> Result<Self, AppError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| AppError::Validation(format!("Invalid path: {}", path.display())))?;
        let content = store.read_file(path_str)?;
        let header: RequirementHeader = serde_yaml::from_str(&content).map_err(|e| {
            AppError::ParseError { what: path.display().to_string(), details: e.to_string() }
        })?;
        Ok(header)
    }
}
