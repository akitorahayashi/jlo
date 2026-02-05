//! Run configuration loading and repository detection.

use super::config_dto::RunConfigDto;
use crate::domain::{AppError, RunConfig, JULES_DIR};
use crate::ports::WorkspaceStore;

/// Load and parse the run configuration.
pub fn load_config(workspace: &impl WorkspaceStore) -> Result<RunConfig, AppError> {
    let config_path = format!("{}/config.toml", JULES_DIR);

    if !workspace.path_exists(&config_path) {
        return Err(AppError::RunConfigMissing);
    }

    let content = workspace.read_file(&config_path)?;
    parse_config_content(&content)
}

/// Parse configuration from string content.
pub fn parse_config_content(content: &str) -> Result<RunConfig, AppError> {
    // Check for legacy [agents] section
    let value: toml::Value = toml::from_str(content)?;
    if value.get("agents").is_some() {
        return Err(AppError::Validation(
            "Legacy [agents] section is not supported. Use workstreams/<name>/scheduled.toml."
                .to_string(),
        ));
    }

    let dto: RunConfigDto = toml::from_str(content)?;
    Ok(RunConfig::from(dto))
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

    Err(AppError::RepositoryDetectionFailed)
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
        let config = parse_config_content(toml).unwrap();

        assert_eq!(config.run.default_branch, "develop");
        assert!(!config.run.parallel);
        assert_eq!(config.run.max_parallel, 5);
        assert_eq!(config.jules.api_url.as_str(), "https://example.com/v1/sessions");
    }

    #[test]
    fn run_config_uses_defaults_for_missing_sections() {
        let toml = r#""#;
        let config = parse_config_content(toml).unwrap();

        assert_eq!(config.run.default_branch, "main");
        assert!(config.run.parallel);
    }

    #[test]
    fn run_config_rejects_agents_section() {
        let toml = r#"
[agents]
observers = ["taxonomy"]
"#;
        let err = parse_config_content(toml).unwrap_err();
        assert!(err.to_string().contains("Legacy [agents] section"));
    }
}
