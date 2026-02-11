//! Run configuration domain models.

use url::Url;

/// Configuration for agent execution loaded from `.jules/config.toml`.
#[derive(Debug, Clone, Default)]
pub struct RunConfig {
    /// Execution configuration.
    pub run: ExecutionConfig,
    /// Jules API configuration.
    pub jules: JulesApiConfig,
}

impl RunConfig {
    /// Validate the configuration, returning an error string if invalid.
    pub fn validate(&self) -> Result<(), String> {
        self.run.validate().map_err(|e| format!("Invalid execution config: {}", e))?;
        self.jules.validate().map_err(|e| format!("Invalid jules api config: {}", e))?;
        Ok(())
    }
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

impl JulesApiConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.timeout_secs == 0 {
            return Err("timeout_secs must be greater than 0".to_string());
        }
        if self.retry_delay_ms == 0 {
            return Err("retry_delay_ms must be greater than 0".to_string());
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
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Default branch for agent operations (implementer works from here).
    pub default_branch: String,
    /// Branch where .jules/ workspace resides.
    pub jules_branch: String,
    /// Whether to run agents in parallel.
    pub parallel: bool,
    /// Maximum number of parallel agent executions.
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

impl ExecutionConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_parallel == 0 {
            return Err("max_parallel must be greater than 0".to_string());
        }
        if self.default_branch.trim().is_empty() {
            return Err("default_branch must not be empty".to_string());
        }
        if self.jules_branch.trim().is_empty() {
            return Err("jules_branch must not be empty".to_string());
        }
        Ok(())
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
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_execution_config_invalid_max_parallel() {
        let config = ExecutionConfig { max_parallel: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_execution_config_empty_branches() {
        let config = ExecutionConfig { default_branch: "  ".to_string(), ..Default::default() };
        assert!(config.validate().is_err());

        let config = ExecutionConfig { jules_branch: "".to_string(), ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_jules_config_invalid_timeout() {
        let config = JulesApiConfig { timeout_secs: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_jules_config_invalid_retry_delay() {
        let config = JulesApiConfig { retry_delay_ms: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }
}
