//! Run command implementation for executing Jules agents.

mod config;
mod multi_role;
mod prompt;
mod single_role;

use std::path::Path;
use std::path::PathBuf;

use crate::domain::{AppError, Layer};

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific roles to run (None = all from config). Only for multi-role layers.
    pub roles: Option<Vec<String>>,
    /// Show assembled prompts without executing.
    pub dry_run: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
    /// Local issue file path (required for single-role layers: planners, implementers).
    pub issue: Option<PathBuf>,
}

/// Result of a run execution.
#[derive(Debug)]
pub struct RunResult {
    /// Roles that were processed.
    pub roles: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Session IDs from Jules (empty if dry_run).
    pub sessions: Vec<String>,
}

/// Execute the run command.
pub fn execute(jules_path: &Path, options: RunOptions) -> Result<RunResult, AppError> {
    // Check if we are in CI environment
    let is_ci = std::env::var("GITHUB_ACTIONS").is_ok();

    // Single-role layers (Planners, Implementers) are issue-driven
    if options.layer.is_single_role() {
        return single_role::execute(
            jules_path,
            options.layer,
            options.issue.as_deref(),
            options.dry_run,
            options.branch.as_deref(),
            is_ci,
        );
    }

    // Multi-role layers (Observers, Deciders)
    multi_role::execute(
        jules_path,
        options.layer,
        options.roles.as_ref(),
        options.dry_run,
        options.branch.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::config::{parse_github_url, resolve_roles};
    use crate::domain::{AgentConfig, AppError, Layer, RunConfig};

    #[test]
    fn resolve_roles_returns_all_when_none_requested() {
        let config = RunConfig {
            agents: AgentConfig {
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
            agents: AgentConfig { observers: vec!["taxonomy".to_string()], ..Default::default() },
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
    fn single_role_layers_are_identified_correctly() {
        assert!(Layer::Planners.is_single_role());
        assert!(Layer::Implementers.is_single_role());
        assert!(!Layer::Observers.is_single_role());
        assert!(!Layer::Deciders.is_single_role());
    }
}
