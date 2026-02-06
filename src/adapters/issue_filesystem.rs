use crate::domain::{AppError, IssueHeader};
use std::path::Path;

/// Reads and parses the header of an issue file.
pub fn read_issue_header(path: &Path) -> Result<IssueHeader, AppError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::IssueFileNotFound(path.display().to_string())
        } else {
            e.into()
        }
    })?;

    serde_yaml::from_str(&content).map_err(|e| AppError::ParseError {
        what: path.display().to_string(),
        details: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_valid_issue_header() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("issue.yml");
        let content = "id: test-1\nrequires_deep_analysis: true\nsource_events: []\n";
        fs::write(&file_path, content).unwrap();

        let header = read_issue_header(&file_path).unwrap();
        assert!(header.requires_deep_analysis);
    }

    #[test]
    fn test_missing_requires_deep_analysis() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("issue.yml");
        let content = "id: test-1\nsource_events: []\n";
        fs::write(&file_path, content).unwrap();

        let result = read_issue_header(&file_path);
        assert!(matches!(result, Err(AppError::ParseError { .. })));
    }

    #[test]
    fn test_invalid_type_requires_deep_analysis() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("issue.yml");
        let content = "id: test-1\nrequires_deep_analysis: \"true\"\nsource_events: []\n";
        fs::write(&file_path, content).unwrap();

        let result = read_issue_header(&file_path);
        assert!(matches!(result, Err(AppError::ParseError { .. })));
    }

    #[test]
    fn test_non_existent_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("non_existent.yml");
        let result = read_issue_header(&file_path);
        assert!(matches!(result, Err(AppError::IssueFileNotFound(_))));
    }

    #[test]
    fn test_malformed_yaml() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("issue.yml");
        let content = "requires_deep_analysis: : true\n";
        fs::write(&file_path, content).unwrap();

        let result = read_issue_header(&file_path);
        assert!(matches!(result, Err(AppError::ParseError { .. })));
    }
}
