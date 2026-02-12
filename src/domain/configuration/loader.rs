//! Run configuration loading and repository detection.

use std::path::Path;

use crate::domain::workspace::paths::jlo;
use crate::domain::{AppError, IoErrorKind, RunConfig, Schedule};
use crate::ports::WorkspaceStore;

/// Load the root schedule from `.jlo/scheduled.toml`.
pub fn load_schedule(store: &impl WorkspaceStore) -> Result<Schedule, AppError> {
    let path = store.jlo_path().join("scheduled.toml");
    let path_str = path.to_string_lossy();

    let content = store.read_file(&path_str).map_err(|err| {
        if matches!(err, AppError::Io { kind: IoErrorKind::NotFound, .. }) {
            AppError::ScheduleConfigMissing(path.display().to_string())
        } else {
            err
        }
    })?;
    Ok(Schedule::parse_toml(&content)?)
}

/// Load and parse the run configuration.
pub fn load_config<W: WorkspaceStore>(
    jules_path: &Path,
    workspace: &W,
) -> Result<RunConfig, AppError> {
    // jules_path is typically .jules/
    // We need to look in .jlo/config.toml which is a sibling of .jules/
    let root = jules_path.parent().ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid .jules path (missing parent): {}",
            jules_path.display()
        ))
    })?;
    let config_path = jlo::config(root);
    let config_path_str = config_path.to_str().ok_or_else(|| {
        AppError::Validation(format!(
            "Config path contains invalid unicode: {}",
            config_path.display()
        ))
    })?;

    if !workspace.file_exists(config_path_str) {
        return Err(AppError::RunConfigMissing);
    }

    let content = workspace.read_file(config_path_str)?;
    parse_config_content(&content)
}

/// Parse configuration from string content.
pub fn parse_config_content(content: &str) -> Result<RunConfig, AppError> {
    let config: RunConfig = toml::from_str(content)?;
    config.validate()?;
    Ok(config)
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
}
