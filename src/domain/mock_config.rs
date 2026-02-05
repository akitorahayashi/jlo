//! Mock execution configuration and output types.

use std::collections::HashMap;

use crate::domain::{AppError, Layer};

/// Configuration for mock execution, loaded from workspace files.
#[derive(Debug, Clone)]
pub struct MockConfig {
    /// Mock scope identifier (embedded in branch names and filenames).
    pub scope: String,
    /// Branch prefixes per layer, from contracts.yml.
    pub branch_prefixes: HashMap<Layer, String>,
    /// Default branch for implementer operations, from config.toml.
    pub default_branch: String,
    /// Jules branch for observer/decider/planner operations, from config.toml.
    pub jules_branch: String,
    /// Allowed issue labels, from github-labels.json.
    pub issue_labels: Vec<String>,
}

impl MockConfig {
    /// Generate branch name for a layer with scope embedded.
    pub fn branch_prefix(&self, layer: Layer) -> Result<&str, AppError> {
        self.branch_prefixes.get(&layer).map(|s| s.as_str()).ok_or_else(|| {
            AppError::Validation(format!("Missing branch_prefix for layer '{}'", layer.dir_name()))
        })
    }

    /// Generate branch name for a layer with scope embedded.
    pub fn branch_name(&self, layer: Layer, suffix: &str) -> Result<String, AppError> {
        let prefix = self.branch_prefix(layer)?;
        Ok(format!("{}{}-{}", prefix, self.scope, suffix))
    }

    /// Get base branch for a layer.
    #[allow(dead_code)]
    pub fn base_branch(&self, layer: Layer) -> &str {
        if layer == Layer::Implementers { &self.default_branch } else { &self.jules_branch }
    }
}

/// Output from mock execution for workflow integration.
#[derive(Debug, Clone)]
pub struct MockOutput {
    /// Branch created by mock execution.
    pub mock_branch: String,
    /// PR number created by mock execution.
    pub mock_pr_number: u64,
    /// PR URL created by mock execution.
    pub mock_pr_url: String,
    /// Mock scope used for this execution.
    pub mock_scope: String,
}

impl MockOutput {
    /// Write outputs to GITHUB_OUTPUT file if set.
    pub fn write_github_output(&self) -> std::io::Result<()> {
        if let Ok(output_file) = std::env::var("GITHUB_OUTPUT") {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().append(true).open(&output_file)?;
            writeln!(file, "mock_branch={}", self.mock_branch)?;
            writeln!(file, "mock_pr_number={}", self.mock_pr_number)?;
            writeln!(file, "mock_pr_url={}", self.mock_pr_url)?;
            writeln!(file, "mock_scope={}", self.mock_scope)?;
        }
        Ok(())
    }

    /// Print outputs in grep-friendly format for local use.
    pub fn print_local(&self) {
        println!("MOCK_BRANCH={}", self.mock_branch);
        println!("MOCK_PR_NUMBER={}", self.mock_pr_number);
        println!("MOCK_PR_URL={}", self.mock_pr_url);
        println!("MOCK_SCOPE={}", self.mock_scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_name_with_scope() {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Observers, "jules-observer-".to_string());
        prefixes.insert(Layer::Implementers, "jules-implementer-".to_string());

        let config = MockConfig {
            scope: "run123".to_string(),
            branch_prefixes: prefixes,
            default_branch: "main".to_string(),
            jules_branch: "jules".to_string(),
            issue_labels: vec!["bugs".to_string()],
        };

        assert_eq!(
            config.branch_name(Layer::Observers, "test").unwrap(),
            "jules-observer-run123-test"
        );
        assert_eq!(
            config.branch_name(Layer::Implementers, "fix").unwrap(),
            "jules-implementer-run123-fix"
        );
    }

    #[test]
    fn base_branch_selection() {
        let config = MockConfig {
            scope: "test".to_string(),
            branch_prefixes: HashMap::new(),
            default_branch: "main".to_string(),
            jules_branch: "jules".to_string(),
            issue_labels: vec![],
        };

        assert_eq!(config.base_branch(Layer::Observers), "jules");
        assert_eq!(config.base_branch(Layer::Deciders), "jules");
        assert_eq!(config.base_branch(Layer::Planners), "jules");
        assert_eq!(config.base_branch(Layer::Implementers), "main");
    }
}
