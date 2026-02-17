//! Pure parse/validate for run configuration (`config.toml`).

use crate::domain::{AppError, RunConfig};

/// Parse and validate run configuration from TOML content.
pub fn parse_config_content(content: &str) -> Result<RunConfig, AppError> {
    let config: RunConfig = toml::from_str(content)?;
    config.validate()?;
    Ok(config)
}

/// Parse a GitHub remote URL to extract `owner/repo`.
///
/// Supports SSH (`git@github.com:owner/repo.git`) and
/// HTTPS (`https://github.com/owner/repo.git`) formats.
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
jlo_target_branch = "develop"
parallel = false
max_parallel = 5

[jules_api]
api_url = "https://example.com/v1/sessions"
timeout_secs = 10
max_retries = 1
retry_delay_ms = 250
"#;
        let config = parse_config_content(toml).unwrap();

        assert_eq!(config.run.jlo_target_branch, "develop");
        assert!(!config.run.parallel);
        assert_eq!(config.run.max_parallel, 5);
        assert_eq!(config.jules_api.api_url.as_str(), "https://example.com/v1/sessions");
    }

    #[test]
    fn run_config_uses_defaults_for_missing_sections() {
        let toml = r#""#;
        let config = parse_config_content(toml).unwrap();

        assert_eq!(config.run.jlo_target_branch, "main");
        assert!(config.run.parallel);
    }

    #[test]
    fn run_config_validation_fails() {
        let toml = r#"
[run]
max_parallel = 0
"#;
        let result = parse_config_content(toml);
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::InvalidConfig(_))));
    }
}
