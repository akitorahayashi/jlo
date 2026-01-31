//! Run configuration loading and repository detection.

use std::fs;
use std::path::Path;

use crate::domain::{AppError, RunConfig};

/// Load and parse the run configuration.
pub fn load_config(jules_path: &Path) -> Result<RunConfig, AppError> {
    let config_path = jules_path.join("config.toml");

    if !config_path.exists() {
        return Err(AppError::RunConfigMissing);
    }

    let content = fs::read_to_string(&config_path)?;
    RunConfig::parse_toml(&content).map_err(AppError::RunConfigInvalid)
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

    // resolve_roles removed (roles are selected via scheduled.toml or explicit CLI roles)
}
