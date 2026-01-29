//! Run command execution logic.

use std::fs;
use std::path::Path;

use crate::domain::{AppError, Layer, RunConfig};
use crate::ports::{AutomationMode, JulesClient, MockJulesClient, SessionRequest};
use crate::services::HttpJulesClient;

/// Options for the run command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Target layer to run.
    pub layer: Layer,
    /// Specific roles to run (None = all from config).
    pub roles: Option<Vec<String>>,
    /// Show assembled prompts without executing.
    pub dry_run: bool,
    /// Run in mock mode (no API calls).
    pub mock: bool,
    /// Override the starting branch.
    pub branch: Option<String>,
}

/// Result of a run execution.
#[derive(Debug)]
pub struct RunResult {
    /// Roles that were processed.
    pub roles: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Session IDs from Jules (empty if dry_run or mock).
    pub sessions: Vec<String>,
}

/// Execute the run command.
pub fn execute(jules_path: &Path, options: RunOptions) -> Result<RunResult, AppError> {
    // Load config
    let config = load_config(jules_path)?;

    // Get roles for the target layer
    let roles = resolve_roles(&config, options.layer, options.roles.as_ref())?;

    if roles.is_empty() {
        println!(
            "No roles configured for layer '{}'. Update .jules/config.toml.",
            options.layer.dir_name()
        );
        return Ok(RunResult { roles: vec![], dry_run: options.dry_run, sessions: vec![] });
    }

    // Determine starting branch
    let starting_branch = options.branch.clone().unwrap_or_else(|| {
        if options.layer == Layer::Implementers {
            config.run.default_branch.clone()
        } else {
            "jules".to_string()
        }
    });

    if options.dry_run {
        execute_dry_run(jules_path, options.layer, &roles, &starting_branch)?;
        return Ok(RunResult { roles, dry_run: true, sessions: vec![] });
    }

    // Determine repository source from git
    let source = detect_repository_source()?;

    // Execute with appropriate client
    let sessions = if options.mock {
        let client = MockJulesClient;
        execute_roles(jules_path, options.layer, &roles, &starting_branch, &source, &client)?
    } else {
        let client = HttpJulesClient::from_env_with_config(&config.jules)?;
        execute_roles(jules_path, options.layer, &roles, &starting_branch, &source, &client)?
    };

    Ok(RunResult { roles, dry_run: false, sessions })
}

/// Execute roles with the given Jules client.
fn execute_roles<C: JulesClient>(
    jules_path: &Path,
    layer: Layer,
    roles: &[String],
    starting_branch: &str,
    source: &str,
    client: &C,
) -> Result<Vec<String>, AppError> {
    let mut sessions = Vec::new();

    for role in roles {
        println!("Executing {} / {}...", layer.dir_name(), role);

        let prompt = assemble_prompt(jules_path, layer, role)?;

        let request = SessionRequest {
            prompt,
            source: source.to_string(),
            starting_branch: starting_branch.to_string(),
            require_plan_approval: false,
            automation_mode: AutomationMode::AutoCreatePr,
        };

        match client.create_session(request) {
            Ok(response) => {
                println!("  ✅ Session created: {}", response.session_id);
                sessions.push(response.session_id);
            }
            Err(e) => {
                println!("  ❌ Failed: {}", e);
                // Continue with other roles even if one fails
            }
        }
    }

    println!("\nCompleted: {}/{} role(s)", sessions.len(), roles.len());
    Ok(sessions)
}

/// Assemble the full prompt for a role.
fn assemble_prompt(jules_path: &Path, layer: Layer, role: &str) -> Result<String, AppError> {
    let role_dir = jules_path.join("roles").join(layer.dir_name()).join(role);
    let prompt_path = role_dir.join("prompt.yml");

    if !prompt_path.exists() {
        return Err(AppError::RoleNotFound(format!(
            "{}/{} (prompt.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    let mut prompt_parts = Vec::new();

    // 1. Read prompt.yml
    let prompt_content = fs::read_to_string(&prompt_path)?;
    prompt_parts.push(prompt_content);

    // 2. Read contracts.yml if it exists
    let contracts_path = jules_path.join("roles").join(layer.dir_name()).join("contracts.yml");
    if contracts_path.exists() {
        let contracts = fs::read_to_string(&contracts_path)?;
        prompt_parts.push(format!("\n---\n# Layer Contracts\n{}", contracts));
    }

    // 3. Read role.yml if exists (observers)
    let role_yml_path = role_dir.join("role.yml");
    if role_yml_path.exists() {
        let role_config = fs::read_to_string(&role_yml_path)?;
        prompt_parts.push(format!("\n---\n# Role Configuration\n{}", role_config));
    }

    // 4. Read notes if directory exists
    let notes_path = role_dir.join("notes");
    if notes_path.exists() && notes_path.is_dir() {
        let mut note_contents = Vec::new();
        for entry in fs::read_dir(&notes_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Ok(content) = fs::read_to_string(&path)
            {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                note_contents.push(format!("## {}\n{}", filename, content));
            }
        }
        if !note_contents.is_empty() {
            prompt_parts.push(format!("\n---\n# Notes\n{}", note_contents.join("\n\n")));
        }
    }

    Ok(prompt_parts.join("\n"))
}

/// Load and parse the run configuration.
fn load_config(jules_path: &Path) -> Result<RunConfig, AppError> {
    let config_path = jules_path.join("config.toml");

    if !config_path.exists() {
        return Err(AppError::RunConfigMissing);
    }

    let content = fs::read_to_string(&config_path)?;
    toml::from_str(&content).map_err(|e| AppError::RunConfigInvalid(e.to_string()))
}

/// Resolve which roles to run for a layer.
fn resolve_roles(
    config: &RunConfig,
    layer: Layer,
    requested: Option<&Vec<String>>,
) -> Result<Vec<String>, AppError> {
    let configured = match layer {
        Layer::Observers => &config.agents.observers,
        Layer::Deciders => &config.agents.deciders,
        Layer::Planners => &config.agents.planners,
        Layer::Implementers => &config.agents.implementers,
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
fn detect_repository_source() -> Result<String, AppError> {
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

/// Execute a dry run, showing assembled prompts.
fn execute_dry_run(
    jules_path: &Path,
    layer: Layer,
    roles: &[String],
    starting_branch: &str,
) -> Result<(), AppError> {
    println!("=== Dry Run: {} ===", layer.display_name());
    println!("Starting branch: {}\n", starting_branch);

    for role in roles {
        println!("--- Role: {} ---", role);

        let role_dir = jules_path.join("roles").join(layer.dir_name()).join(role);
        let prompt_path = role_dir.join("prompt.yml");

        if !prompt_path.exists() {
            println!("  ⚠️  prompt.yml not found at {}\n", prompt_path.display());
            continue;
        }

        // Read contracts.yml for the layer
        let contracts_path = jules_path.join("roles").join(layer.dir_name()).join("contracts.yml");

        println!("  Prompt: {}", prompt_path.display());
        if contracts_path.exists() {
            println!("  Contracts: {}", contracts_path.display());
        }

        // Show role.yml if exists (observers only)
        let role_yml_path = role_dir.join("role.yml");
        if role_yml_path.exists() {
            println!("  Role config: {}", role_yml_path.display());
        }

        // Show notes directory if exists
        let notes_path = role_dir.join("notes");
        if notes_path.exists() {
            let note_count = fs::read_dir(&notes_path)
                .map(|entries| entries.filter(|e| e.is_ok()).count())
                .unwrap_or(0);
            println!("  Notes: {} files", note_count);
        }

        // Show assembled prompt length
        if let Ok(prompt) = assemble_prompt(jules_path, layer, role) {
            println!("  Assembled prompt: {} chars", prompt.len());
        }

        println!();
    }

    println!("Total: {} role(s) would be executed", roles.len());
    Ok(())
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
}
