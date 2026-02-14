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
        self.run.validate()?;
        self.jules.validate()?;
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

impl JulesApiConfig {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.timeout_secs == 0 {
            return Err(AppError::InvalidConfig("timeout_secs must be greater than 0".to_string()));
        }
        if self.max_retries == 0 {
            return Err(AppError::InvalidConfig("max_retries must be greater than 0".to_string()));
        }
        if self.retry_delay_ms == 0 {
            return Err(AppError::InvalidConfig(
                "retry_delay_ms must be greater than 0".to_string(),
            ));
        }
        Ok(())
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
    /// Target branch for agent operations (base for PRs).
    #[serde(default = "default_jlo_target_branch")]
    pub jlo_target_branch: String,
    /// Branch where .jules/ workspace resides (worker).
    #[serde(default = "default_jules_worker_branch")]
    pub jules_worker_branch: String,
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
            jlo_target_branch: default_jlo_target_branch(),
            jules_worker_branch: default_jules_worker_branch(),
            parallel: default_true(),
            max_parallel: default_max_parallel(),
        }
    }
}

impl ExecutionConfig {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.max_parallel == 0 {
            return Err(AppError::InvalidConfig("max_parallel must be greater than 0".to_string()));
        }
        if self.jlo_target_branch.trim().is_empty() {
            return Err(AppError::InvalidConfig("jlo_target_branch must not be empty".to_string()));
        }
        if self.jules_worker_branch.trim().is_empty() {
            return Err(AppError::InvalidConfig(
                "jules_worker_branch must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

fn default_jlo_target_branch() -> String {
    "main".to_string()
}

fn default_jules_worker_branch() -> String {
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
        assert_eq!(config.run.jlo_target_branch, "main");
        assert_eq!(config.run.jules_worker_branch, "jules");
        assert!(config.run.parallel);
        assert_eq!(config.run.max_parallel, 3);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_execution_config_invalid_max_parallel() {
        let config = ExecutionConfig { max_parallel: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_execution_config_empty_branches() {
        let config = ExecutionConfig { jlo_target_branch: "  ".to_string(), ..Default::default() };
        assert!(config.validate().is_err());

        let config = ExecutionConfig { jules_worker_branch: "".to_string(), ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_jules_config_invalid_timeout() {
        let config = JulesApiConfig { timeout_secs: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_jules_config_invalid_max_retries() {
        let config = JulesApiConfig { max_retries: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_jules_config_invalid_retry_delay() {
        let config = JulesApiConfig { retry_delay_ms: 0, ..Default::default() };
        assert!(config.validate().is_err());
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
        assert!(matches!(err, AppError::InvalidConfig(msg) if msg.contains("max_parallel")));
    }

    #[test]
    fn validate_rejects_zero_timeout() {
        let mut config = RunConfig::default();
        config.jules.timeout_secs = 0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, AppError::InvalidConfig(msg) if msg.contains("timeout_secs")));
    }
}
