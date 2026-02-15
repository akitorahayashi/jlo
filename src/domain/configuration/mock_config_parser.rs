//! Pure parse/validate for mock configuration artifacts.

use serde::Deserialize;

use crate::domain::AppError;

#[derive(Deserialize)]
struct ContractConfig {
    branch_prefix: String,
}

/// Extract `branch_prefix` from a contracts.yml content string.
pub fn extract_branch_prefix(content: &str) -> Result<String, AppError> {
    let config: ContractConfig = serde_yaml::from_str(content).map_err(|e| {
        AppError::ParseError { what: "contracts.yml".to_string(), details: e.to_string() }
    })?;

    if config.branch_prefix.trim().is_empty() {
        return Err(AppError::InvalidConfig("branch_prefix cannot be empty".to_string()));
    }

    Ok(config.branch_prefix)
}

/// Extract issue label names from a `github-labels.json` content string.
pub fn extract_issue_labels(content: &str) -> Result<Vec<String>, AppError> {
    let json: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        AppError::ParseError { what: "github-labels.json".to_string(), details: e.to_string() }
    })?;

    let labels = json
        .get("issue_labels")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    Ok(labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_branch_prefix() {
        let content = r#"
layer: observers
branch_prefix: jules-observer-
constraints:
  - Do NOT write to issues/
"#;
        assert_eq!(extract_branch_prefix(content).unwrap(), "jules-observer-".to_string());
    }

    #[test]
    fn test_extract_branch_prefix_with_quotes() {
        let content = r#"branch_prefix: "jules-test-""#;
        assert_eq!(extract_branch_prefix(content).unwrap(), "jules-test-".to_string());
    }

    #[test]
    fn test_extract_branch_prefix_missing() {
        let content = r#"layer: observers"#;
        assert!(extract_branch_prefix(content).is_err());
    }

    #[test]
    fn test_extract_branch_prefix_empty() {
        let content = r#"branch_prefix: """#;
        assert!(extract_branch_prefix(content).is_err());
    }

    #[test]
    fn test_extract_issue_labels() {
        let content = r#"{
            "issue_labels": {
                "bugs": {"color": "d73a4a"},
                "feats": {"color": "ff6600"}
            }
        }"#;
        let labels = extract_issue_labels(content).unwrap();
        assert!(labels.contains(&"bugs".to_string()));
        assert!(labels.contains(&"feats".to_string()));
    }
}
