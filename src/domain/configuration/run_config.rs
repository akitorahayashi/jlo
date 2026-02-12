//! Run configuration domain models.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::domain::AppError;

/// Configuration for agent execution loaded from `.jules/config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    /// Execution configuration.
    #[serde(default)]
    pub run: ExecutionConfig,
    /// Jules API configuration.
    #[serde(default)]
    pub jules: JulesApiConfig,
    /// Workflow timing configuration.
    #[serde(default)]
    #[allow(dead_code)]
    pub workflow: WorkflowTimingConfig,
}

impl RunConfig {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.run.max_parallel == 0 {
            return Err(AppError::Validation("max_parallel must be greater than 0".to_string()));
        }
        if self.jules.timeout_secs == 0 {
            return Err(AppError::Validation("timeout_secs must be greater than 0".to_string()));
        }
        Ok(())
    }
}

/// Jules API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Execution configuration for agent runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionConfig {
    /// Default branch for agent operations (implementer works from here).
    #[serde(default = "default_branch")]
    pub default_branch: String,
    /// Branch where .jules/ workspace resides.
    #[serde(default = "default_jules_branch")]
    pub jules_branch: String,
    /// Whether to run agents in parallel.
    #[serde(default = "default_true")]
    pub parallel: bool,
    /// Maximum number of parallel agent executions.
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
}

impl Default for ExecutionConfig {
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

/// Workflow timing configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowTimingConfig {
    pub runner_mode: Option<String>,
    pub cron: Option<Vec<String>>,
    pub wait_minutes_default: Option<u32>,
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
    fn validate_accepts_valid_config() {
        let config = RunConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_zero_max_parallel() {
        let mut config = RunConfig::default();
        config.run.max_parallel = 0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, AppError::Validation(msg) if msg.contains("max_parallel")));
    }

    #[test]
    fn validate_rejects_zero_timeout() {
        let mut config = RunConfig::default();
        config.jules.timeout_secs = 0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, AppError::Validation(msg) if msg.contains("timeout_secs")));
    }
}
