use crate::domain::{AppError, IssueHeader};
use std::path::Path;

/// Reads and parses the header of an issue file.
pub fn read_issue_header(path: &Path) -> Result<IssueHeader, AppError> {
    let content = std::fs::read_to_string(path)?;
    let header: IssueHeader = serde_yaml::from_str(&content).map_err(|e| AppError::ParseError {
        what: path.display().to_string(),
        details: e.to_string(),
    })?;
    Ok(header)
}
