//! Run configuration loading and role resolution.

use std::fs;
use std::path::Path;

use serde::Deserialize;
use url::Url;

use crate::domain::{AgentConfig, AppError, JulesApiConfig, Layer, RunConfig, RunSettings};

/// Load and parse the run configuration.
pub fn load_config(jules_path: &Path) -> Result<RunConfig, AppError> {
    let config_path = jules_path.join("config.toml");

    if !config_path.exists() {
        return Err(AppError::RunConfigMissing);
    }

    let content = fs::read_to_string(&config_path)?;
    let dto: RunConfigDto =
        toml::from_str(&content).map_err(|e| AppError::RunConfigInvalid(e.to_string()))?;

    dto.try_into()
}

// --- DTOs for TOML deserialization ---

#[derive(Debug, Deserialize)]
struct RunConfigDto {
    #[serde(default)]
    agents: AgentConfigDto,
    #[serde(default)]
    run: RunSettingsDto,
    #[serde(default)]
    jules: JulesApiConfigDto,
}

#[derive(Debug, Default, Deserialize)]
struct AgentConfigDto {
    #[serde(default)]
    observers: Vec<String>,
    #[serde(default)]
    deciders: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct JulesApiConfigDto {
    #[serde(default = "default_api_url")]
    api_url: String,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    max_retries: u32,
    #[serde(default = "default_retry_delay_ms")]
    retry_delay_ms: u64,
}

impl Default for JulesApiConfigDto {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
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

fn default_retry_delay_ms() -> u64 {
    1000
}

#[derive(Debug, Deserialize)]
struct RunSettingsDto {
    #[serde(default = "default_branch")]
    default_branch: String,
    #[serde(default = "default_jules_branch")]
    jules_branch: String,
    #[serde(default = "default_true")]
    parallel: bool,
    #[serde(default = "default_max_parallel")]
    max_parallel: usize,
}

impl Default for RunSettingsDto {
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

impl TryFrom<RunConfigDto> for RunConfig {
    type Error = AppError;

    fn try_from(dto: RunConfigDto) -> Result<Self, Self::Error> {
        Ok(RunConfig {
            agents: AgentConfig {
                observers: dto.agents.observers,
                deciders: dto.agents.deciders,
            },
            run: RunSettings {
                default_branch: dto.run.default_branch,
                jules_branch: dto.run.jules_branch,
                parallel: dto.run.parallel,
                max_parallel: dto.run.max_parallel,
            },
            jules: JulesApiConfig {
                api_url: Url::parse(&dto.jules.api_url)
                    .map_err(|e| AppError::RunConfigInvalid(format!("Invalid API URL: {}", e)))?,
                timeout_secs: dto.jules.timeout_secs,
                max_retries: dto.jules.max_retries,
                retry_delay_ms: dto.jules.retry_delay_ms,
            },
        })
    }
}

// --- End DTOs ---

/// Resolve which roles to run for a multi-role layer.
///
/// Only Observers and Deciders support role configuration.
/// Single-role layers (Planners, Implementers) should not call this function.
pub fn resolve_roles(
    config: &RunConfig,
    layer: Layer,
    requested: Option<&Vec<String>>,
) -> Result<Vec<String>, AppError> {
    let configured = match layer {
        Layer::Observers => &config.agents.observers,
        Layer::Deciders => &config.agents.deciders,
        Layer::Planners | Layer::Implementers => {
            // Single-role layers should not reach here
            return Err(AppError::ConfigError(format!(
                "Layer '{}' is single-role and does not use role configuration",
                layer.dir_name()
            )));
        }
    };

    match requested {
        Some(roles) => {
            // Validate that requested roles exist in config
            for role in roles {
                if !configured.contains(role) {
                    return Err(AppError::RoleNotInConfig {
                        role: role.clone(),
                        layer: layer.dir_name().to_string(),
                    });
                }
            }
            Ok(roles.clone())
        }
        None => Ok(configured.clone()),
    }
}

/// Detect the repository source from git remote.
pub fn detect_repository_source() -> Result<String, AppError> {
    // Try to read from git config
    let output = std::process::Command::new("git").args(["remote", "get-url", "origin"]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let url = String::from_utf8_lossy(&output.stdout);
        // Parse GitHub URL: git@github.com:owner/repo.git or https://github.com/owner/repo.git
        if let Some(repo) = parse_github_url(url.trim()) {
            return Ok(format!("sources/github/{}", repo));
        }
    }

    // Fallback to environment variable
    if let Ok(repo) = std::env::var("GITHUB_REPOSITORY") {
        return Ok(format!("sources/github/{}", repo));
    }

    Err(AppError::ConfigError(
        "Could not detect repository. Set GITHUB_REPOSITORY or run from a git repository.".into(),
    ))
}

/// Parse a GitHub URL to extract owner/repo.
fn parse_github_url(url: &str) -> Option<String> {
    // SSH: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let repo = rest.trim_end_matches(".git");
        return Some(repo.to_string());
    }

    // HTTPS: https://github.com/owner/repo.git
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let repo = rest.trim_end_matches(".git");
        return Some(repo.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_roles_returns_all_when_none_requested() {
        let config = RunConfig {
            agents: crate::domain::AgentConfig {
                observers: vec!["taxonomy".to_string(), "qa".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let roles = resolve_roles(&config, Layer::Observers, None).unwrap();
        assert_eq!(roles, vec!["taxonomy", "qa"]);
    }

    #[test]
    fn resolve_roles_validates_requested_roles() {
        let config = RunConfig {
            agents: crate::domain::AgentConfig {
                observers: vec!["taxonomy".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let requested = vec!["nonexistent".to_string()];
        let result = resolve_roles(&config, Layer::Observers, Some(&requested));
        assert!(matches!(result, Err(AppError::RoleNotInConfig { .. })));
    }

    #[test]
    fn parse_github_url_ssh() {
        let result = parse_github_url("git@github.com:owner/repo.git");
        assert_eq!(result, Some("owner/repo".to_string()));
    }

    #[test]
    fn parse_github_url_https() {
        let result = parse_github_url("https://github.com/owner/repo.git");
        assert_eq!(result, Some("owner/repo".to_string()));
    }

    #[test]
    fn parse_github_url_invalid() {
        let result = parse_github_url("https://gitlab.com/owner/repo.git");
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_roles_rejects_single_role_layers() {
        let config = RunConfig::default();

        let result = resolve_roles(&config, Layer::Planners, None);
        assert!(matches!(result, Err(AppError::ConfigError(_))));

        let result = resolve_roles(&config, Layer::Implementers, None);
        assert!(matches!(result, Err(AppError::ConfigError(_))));
    }

    #[test]
    fn run_config_parses_from_toml() {
        let toml = r#"
[agents]
observers = ["taxonomy", "qa"]
deciders = ["triage_generic"]

[run]
default_branch = "develop"
parallel = false
max_parallel = 5
"#;
        let dto: RunConfigDto = toml::from_str(toml).unwrap();
        let config: RunConfig = dto.try_into().unwrap();

        assert_eq!(config.agents.observers, vec!["taxonomy", "qa"]);
        assert_eq!(config.agents.deciders, vec!["triage_generic"]);
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
        let dto: RunConfigDto = toml::from_str(toml).unwrap();
        let config: RunConfig = dto.try_into().unwrap();

        assert_eq!(config.agents.observers, vec!["test"]);
        assert_eq!(config.run.default_branch, "main");
        assert!(config.run.parallel);
    }
}
