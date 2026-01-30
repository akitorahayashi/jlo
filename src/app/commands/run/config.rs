//! Run configuration loading and role resolution.

use std::fs;
use std::path::Path;

use crate::domain::{AppError, Layer, RunConfig};

/// Load and parse the run configuration.
pub fn load_config(jules_path: &Path) -> Result<RunConfig, AppError> {
    let config_path = jules_path.join("config.toml");

    if !config_path.exists() {
        return Err(AppError::RunConfigMissing);
    }

    let content = fs::read_to_string(&config_path)?;
    toml::from_str(&content).map_err(|e| AppError::RunConfigInvalid(e.to_string()))
}

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
pub fn parse_github_url(url: &str) -> Option<String> {
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
}
