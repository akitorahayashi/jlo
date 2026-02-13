use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use serde::Deserialize;

use crate::domain::configuration::loader::load_config;
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, MockConfig, RunOptions};
use crate::ports::WorkspaceStore;

#[derive(Deserialize)]
struct ContractConfig {
    branch_prefix: String,
}

/// Validate prerequisites for mock mode.
pub fn validate_mock_prerequisites(_options: &RunOptions) -> Result<(), AppError> {
    // Check for GH_TOKEN
    if std::env::var("GH_TOKEN").is_err() {
        return Err(AppError::MissingArgument(
            "Mock mode requires GH_TOKEN environment variable to be set".to_string(),
        ));
    }

    // Check for required tools
    if std::process::Command::new("git").arg("--version").output().is_err() {
        return Err(AppError::ExternalToolError {
            tool: "git".to_string(),
            error: "git is required for mock mode but not found in PATH".to_string(),
        });
    }

    if std::process::Command::new("gh").arg("--version").output().is_err() {
        return Err(AppError::ExternalToolError {
            tool: "gh".to_string(),
            error: "gh CLI is required for mock mode but not found in PATH".to_string(),
        });
    }

    Ok(())
}

fn load_branch_prefix_for_layer<W: WorkspaceStore>(
    jules_path: &Path,
    layer: Layer,
    workspace: &W,
) -> Result<String, AppError> {
    let contracts_path = jules::contracts(jules_path, layer);
    let contracts_path_str = contracts_path
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Invalid contracts path".to_string()))?;

    let content = workspace.read_file(contracts_path_str).map_err(|_| {
        AppError::InvalidConfig(format!(
            "Missing contracts file for layer '{}' at {}",
            layer.dir_name(),
            contracts_path.display()
        ))
    })?;

    extract_branch_prefix(&content).map_err(|e| {
        AppError::InvalidConfig(format!(
            "Invalid contracts file for layer '{}' at {}: {}",
            layer.dir_name(),
            contracts_path.display(),
            e
        ))
    })
}

/// Load mock configuration from workspace files.
pub fn load_mock_config<W: WorkspaceStore>(
    jules_path: &Path,
    _options: &RunOptions,
    workspace: &W,
) -> Result<MockConfig, AppError> {
    // Load run config for branch settings
    let run_config = load_config(jules_path, workspace)?;

    // Load branch prefixes from layer contracts.
    let mut branch_prefixes = HashMap::new();
    for layer in Layer::ALL {
        let prefix = load_branch_prefix_for_layer(jules_path, layer, workspace)?;
        branch_prefixes.insert(layer, prefix);
    }

    // Load issue labels from github-labels.json
    let labels_path = jules::github_labels(jules_path);
    let labels_path_str = labels_path
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Invalid labels path".to_string()))?;
    let labels_content = workspace.read_file(labels_path_str).map_err(|_| {
        AppError::InvalidConfig(format!(
            "Missing github-labels.json for mock mode: {}",
            labels_path.display()
        ))
    })?;
    let issue_labels = extract_issue_labels(&labels_content)?;
    if issue_labels.is_empty() {
        return Err(AppError::InvalidConfig(format!(
            "No issue labels defined in github-labels.json: {}",
            labels_path.display()
        )));
    }

    // Generate mock tag if not provided
    // Generate mock tag: env var -> CI default -> local default
    let mock_tag = std::env::var("JULES_MOCK_TAG").ok().unwrap_or_else(|| {
        let prefix = if std::env::var("GITHUB_ACTIONS").is_ok() { "mock-ci" } else { "mock-local" };
        let generated = format!("{}-{}", prefix, Utc::now().format("%Y%m%d%H%M%S"));
        println!("Mock tag not set; using {}", generated);
        generated
    });

    if !mock_tag.contains("mock") {
        return Err(AppError::InvalidConfig(
            "JULES_MOCK_TAG must include 'mock' to mark mock artifacts.".to_string(),
        ));
    }
    if !validate_safe_path_component(&mock_tag) {
        return Err(AppError::InvalidConfig(
            "JULES_MOCK_TAG must be a safe path component (letters, numbers, '-' or '_')."
                .to_string(),
        ));
    }

    Ok(MockConfig {
        mock_tag,
        branch_prefixes,
        jlo_target_branch: run_config.run.jlo_target_branch,
        jules_worker_branch: run_config.run.jules_worker_branch,
        issue_labels,
    })
}

/// Extract branch_prefix from contracts.yml content.
fn extract_branch_prefix(content: &str) -> Result<String, AppError> {
    let config: ContractConfig = serde_yaml::from_str(content).map_err(|e| {
        AppError::ParseError { what: "contracts.yml".to_string(), details: e.to_string() }
    })?;

    if config.branch_prefix.trim().is_empty() {
        return Err(AppError::InvalidConfig("branch_prefix cannot be empty".to_string()));
    }

    Ok(config.branch_prefix)
}

/// Extract issue labels from github-labels.json content.
fn extract_issue_labels(content: &str) -> Result<Vec<String>, AppError> {
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

    #[test]
    fn load_branch_prefix_for_innovators_uses_contracts_yml() {
        use crate::testing::MockWorkspaceStore;

        let workspace = MockWorkspaceStore::new().with_file(
            ".jules/roles/innovators/contracts.yml",
            "branch_prefix: jules-innovator-\n",
        );

        let prefix =
            load_branch_prefix_for_layer(Path::new(".jules"), Layer::Innovators, &workspace)
                .unwrap();

        assert_eq!(prefix, "jules-innovator-");
    }

    #[test]
    fn rejects_mock_tag_with_path_separator() {
        let mock_tag = "mock-../escape";
        assert!(mock_tag.contains("mock"));
        assert!(!validate_safe_path_component(mock_tag));
    }

    #[test]
    fn rejects_mock_tag_with_newline() {
        let mock_tag = "mock-run\ninjected";
        assert!(mock_tag.contains("mock"));
        assert!(!validate_safe_path_component(mock_tag));
    }
}
