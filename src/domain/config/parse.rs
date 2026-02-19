//! Pure parse/validate for run configuration (`config.toml`).

use crate::domain::{AppError, ControlPlaneConfig};

/// Parse and validate run configuration from TOML content.
pub fn parse_config_content(content: &str) -> Result<ControlPlaneConfig, AppError> {
    let config: ControlPlaneConfig = toml::from_str(content)?;
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

[jules_api]
api_url = "https://example.com/v1/sessions"
timeout_secs = 10
max_retries = 1
retry_delay_ms = 250
"#;
        let config = parse_config_content(toml).unwrap();

        assert_eq!(config.run.jlo_target_branch, "develop");
        assert_eq!(config.jules_api.api_url.as_str(), "https://example.com/v1/sessions");
    }

    #[test]
    fn run_config_uses_defaults_for_missing_sections() {
        let toml = r#""#;
        let config = parse_config_content(toml).unwrap();

        assert_eq!(config.run.jlo_target_branch, "main");
    }

    #[test]
    fn run_config_validation_fails() {
        let toml = r#"
[run]
jlo_target_branch = ""
"#;
        let result = parse_config_content(toml);
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::Config(_))));
    }

    #[test]
    fn run_config_rejects_removed_parallel_fields() {
        let toml = r#"
[run]
parallel = true
max_parallel = 3
"#;
        let result = parse_config_content(toml);
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::TomlParseError(_))));
    }
}
