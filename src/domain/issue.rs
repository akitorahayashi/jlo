use serde::Deserialize;
use std::path::Path;

use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Header fields for an issue.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueHeader {
    /// Whether the issue requires deep analysis (planner) or implementation (implementer).
    pub requires_deep_analysis: bool,
}

impl IssueHeader {
    /// Read issue header from a file in the workspace.
    pub fn read(store: &impl WorkspaceStore, path: &Path) -> Result<Self, AppError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| AppError::Validation(format!("Invalid path: {}", path.display())))?;
        let content = store.read_file(path_str)?;
        let header: IssueHeader = serde_yaml::from_str(&content).map_err(|e| {
            AppError::ParseError { what: path.display().to_string(), details: e.to_string() }
        })?;
        Ok(header)
    }
}
