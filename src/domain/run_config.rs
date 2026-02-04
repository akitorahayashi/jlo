//! Run configuration domain models.


use url::Url;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum RunConfigError {
    #[error("Legacy [agents] section is not supported. Use workstreams/<name>/scheduled.toml.")]
    LegacyAgentSection,

    #[error("Run config invalid: {0}")]
    ConfigInvalid(String),

    #[error("TOML format error: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Configuration for agent execution loaded from `.jules/config.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    /// Execution settings.
    #[serde(default)]
    pub run: RunSettings,
    /// Jules API settings.
    #[serde(default)]
    pub jules: JulesApiConfig,
}

impl RunConfig {
    /// Parse configuration from TOML content.
    pub fn parse_toml(content: &str) -> Result<Self, RunConfigError> {
        let value: toml::Value = toml::from_str(content)?;
        if value.get("agents").is_some() {
            return Err(RunConfigError::LegacyAgentSection);
        }
        toml::from_str(content).map_err(RunConfigError::Toml)
    }
}

/// Jules API configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JulesApiConfig {
    /// Jules API endpoint URL.
    #[serde(default = "default_api_url")]
    pub api_url: Url,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum retry attempts.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Delay between retries in milliseconds.
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

impl Default for JulesApiConfig {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
        }
    }
}

fn default_api_url() -> Url {
    Url::parse("https://jules.googleapis.com/v1alpha/sessions")
        .expect("Default API URL must be valid")
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

/// Execution settings for agent runs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSettings {
    /// Default branch for agent operations (implementers work from here).
    #[serde(default = "default_branch")]
    pub default_branch: String,
    /// Branch where .jules/ workspace resides.
    #[serde(default = "default_jules_branch")]
    pub jules_branch: String,
    /// Whether to run agents in parallel.
    #[allow(dead_code)]
    #[serde(default = "default_true")]
    pub parallel: bool,
    /// Maximum number of parallel agent executions.
    #[allow(dead_code)]
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
}

impl Default for RunSettings {
    fn default() -> Self {
        Self {
            default_branch: default_branch(),
            jules_branch: default_jules_branch(),
            parallel: default_true(),
            max_parallel: default_max_parallel(),
        }
    }
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_jules_branch() -> String {
    "jules".to_string()
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
        assert_eq!(config.run.default_branch, "main");
        assert_eq!(config.run.jules_branch, "jules");
        assert!(config.run.parallel);
        assert_eq!(config.run.max_parallel, 3);
    }

    #[test]
    fn run_config_parses_from_toml() {
        let toml = r#"
[run]
default_branch = "develop"
parallel = false
max_parallel = 5

[jules]
api_url = "https://example.com/v1/sessions"
timeout_secs = 10
max_retries = 1
retry_delay_ms = 250
"#;
        let config = RunConfig::parse_toml(toml).unwrap();

        assert_eq!(config.run.default_branch, "develop");
        assert!(!config.run.parallel);
        assert_eq!(config.run.max_parallel, 5);
        assert_eq!(config.jules.api_url.as_str(), "https://example.com/v1/sessions");
    }

    #[test]
    fn run_config_uses_defaults_for_missing_sections() {
        let toml = r#""#;
        let config = RunConfig::parse_toml(toml).unwrap();

        assert_eq!(config.run.default_branch, "main");
        assert!(config.run.parallel);
    }

    #[test]
    fn run_config_rejects_agents_section() {
        let toml = r#"
[agents]
observers = ["taxonomy"]
"#;
        let err = RunConfig::parse_toml(toml).unwrap_err();
        assert!(err.to_string().contains("Legacy [agents] section"));
    }
}
