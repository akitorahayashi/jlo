//! Run configuration domain models.

use url::Url;

/// Configuration for agent execution loaded from `.jules/config.toml`.
#[derive(Debug, Clone, Default)]
pub struct RunConfig {
    /// Agent role assignments per layer.
    pub agents: AgentConfig,
    /// Execution settings.
    pub run: RunSettings,
    /// Jules API settings.
    pub jules: JulesApiConfig,
}

/// Agent role assignments per layer.
///
/// Only multi-role layers (observers, deciders) are configured here.
/// Single-role layers (planners, implementers) are issue-driven and
/// do not require role configuration.
#[derive(Debug, Clone, Default)]
pub struct AgentConfig {
    /// Observer role names.
    pub observers: Vec<String>,
    /// Decider role names.
    pub deciders: Vec<String>,
}

/// Jules API configuration.
#[derive(Debug, Clone)]
pub struct JulesApiConfig {
    /// Jules API endpoint URL.
    pub api_url: Url,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Delay between retries in milliseconds.
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
    Url::parse("https://jules.googleapis.com/v1alpha/sessions").expect("Default URL must be valid")
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
#[derive(Debug, Clone)]
pub struct RunSettings {
    /// Default branch for agent operations (implementers work from here).
    pub default_branch: String,
    /// Branch where .jules/ workspace resides.
    pub jules_branch: String,
    /// Whether to run agents in parallel.
    pub parallel: bool,
    /// Maximum number of parallel agent executions.
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
        assert!(config.agents.observers.is_empty());
        assert!(config.agents.deciders.is_empty());
        assert_eq!(config.run.default_branch, "main");
        assert_eq!(config.run.jules_branch, "jules");
        assert!(config.run.parallel);
        assert_eq!(config.run.max_parallel, 3);
        assert_eq!(
            config.jules.api_url.as_str(),
            "https://jules.googleapis.com/v1alpha/sessions"
        );
    }
}
