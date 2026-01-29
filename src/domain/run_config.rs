//! Run configuration domain models.

use serde::Deserialize;

/// Configuration for agent execution loaded from `.jules/config.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RunConfig {
    /// Agent role assignments per layer.
    #[serde(default)]
    pub agents: AgentConfig,
    /// Execution settings.
    #[serde(default)]
    pub run: RunSettings,
    /// Jules API settings.
    #[serde(default)]
    pub jules: JulesApiConfig,
}

/// Agent role assignments per layer.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentConfig {
    /// Observer role names.
    #[serde(default)]
    pub observers: Vec<String>,
    /// Decider role names.
    #[serde(default)]
    pub deciders: Vec<String>,
    /// Planner role names.
    #[serde(default)]
    pub planners: Vec<String>,
    /// Implementer role names.
    #[serde(default)]
    pub implementers: Vec<String>,
}

/// Jules API configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct JulesApiConfig {
    /// Jules API endpoint URL.
    #[serde(default = "default_api_url")]
    pub api_url: String,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum retry attempts.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for JulesApiConfig {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

fn default_api_url() -> String {
    "https://jules.googleapis.com/v1alpha/sessions".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

/// Execution settings for agent runs.
#[derive(Debug, Clone, Deserialize)]
pub struct RunSettings {
    /// Default branch for agent operations.
    #[serde(default = "default_branch")]
    pub default_branch: String,
    /// Whether to run agents in parallel.
    #[serde(default = "default_true")]
    pub parallel: bool,
    /// Maximum number of parallel agent executions.
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
}

impl Default for RunSettings {
    fn default() -> Self {
        Self {
            default_branch: default_branch(),
            parallel: default_true(),
            max_parallel: default_max_parallel(),
        }
    }
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_true() -> bool {
    true
}

fn default_max_parallel() -> usize {
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_config_defaults() {
        let config = RunConfig::default();
        assert!(config.agents.observers.is_empty());
        assert_eq!(config.run.default_branch, "main");
        assert!(config.run.parallel);
        assert_eq!(config.run.max_parallel, 3);
    }

    #[test]
    fn run_config_parses_from_toml() {
        let toml = r#"
[agents]
observers = ["taxonomy", "qa"]
deciders = ["triage"]

[run]
default_branch = "develop"
parallel = false
max_parallel = 5
"#;
        let config: RunConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.agents.observers, vec!["taxonomy", "qa"]);
        assert_eq!(config.agents.deciders, vec!["triage"]);
        assert!(config.agents.planners.is_empty());
        assert_eq!(config.run.default_branch, "develop");
        assert!(!config.run.parallel);
        assert_eq!(config.run.max_parallel, 5);
    }

    #[test]
    fn run_config_uses_defaults_for_missing_sections() {
        let toml = r#"
[agents]
observers = ["test"]
"#;
        let config: RunConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.agents.observers, vec!["test"]);
        assert_eq!(config.run.default_branch, "main");
        assert!(config.run.parallel);
    }
}
