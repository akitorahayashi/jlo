//! Multi-role layer execution (Observers, Deciders).

use std::fs;
use std::path::Path;

use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::prompt::assemble_prompt;
use super::role_selection::{RoleSelectionInput, select_roles};
use crate::domain::{AppError, Layer};
use crate::ports::{AutomationMode, JulesClient, SessionRequest};
use crate::services::HttpJulesClient;

/// Execute a multi-role layer (Observers or Deciders).
pub fn execute(
    jules_path: &Path,
    layer: Layer,
    roles: Option<&Vec<String>>,
    workstream: Option<&str>,
    scheduled: bool,
    dry_run: bool,
    branch: Option<&str>,
) -> Result<RunResult, AppError> {
    // Load config
    let config = load_config(jules_path)?;

    let workstream = workstream.ok_or_else(|| {
        AppError::config_error("Workstream is required for observers and deciders")
    })?;

    if scheduled && roles.is_some() {
        return Err(AppError::config_error("Cannot combine --scheduled with --role"));
    }
    if !scheduled && roles.is_none() {
        return Err(AppError::config_error("Manual mode requires --role (or use --scheduled)"));
    }

    let resolved_roles = select_roles(RoleSelectionInput {
        jules_path,
        layer,
        workstream,
        scheduled,
        requested_roles: roles,
    })?;

    if resolved_roles.is_empty() {
        println!(
            "No roles configured for layer '{}' in workstream '{}'.",
            layer.dir_name(),
            workstream
        );
        return Ok(RunResult { roles: vec![], dry_run, sessions: vec![] });
    }

    // Determine starting branch (multi-role layers always use jules branch)
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_branch.clone());

    if dry_run {
        execute_dry_run(jules_path, layer, &resolved_roles, &starting_branch)?;
        return Ok(RunResult { roles: resolved_roles, dry_run: true, sessions: vec![] });
    }

    // Determine repository source from git
    let source = detect_repository_source()?;

    // Execute with appropriate client
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let sessions =
        execute_roles(jules_path, layer, &resolved_roles, &starting_branch, &source, &client)?;

    Ok(RunResult { roles: resolved_roles, dry_run: false, sessions })
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
    let mut failures = 0;

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
                failures += 1;
            }
        }
    }

    println!("\nCompleted: {}/{} role(s)", sessions.len(), roles.len());

    if failures > 0 {
        return Err(AppError::ConfigError(format!(
            "{} of {} roles failed to execute",
            failures,
            roles.len()
        )));
    }

    Ok(sessions)
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
