//! Single-role layer execution (Planners, Implementers).

use std::fs;
use std::path::Path;

use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::prompt::assemble_single_role_prompt;
use crate::domain::{AppError, Layer};
use crate::ports::{AutomationMode, JulesClient, SessionRequest};
use crate::services::adapters::jules_client_http::HttpJulesClient;

const PLANNER_WORKFLOW_NAME: &str = "jules-run-planner.yml";
const IMPLEMENTER_WORKFLOW_NAME: &str = "jules-run-implementer.yml";

/// Execute a single-role layer (Planners or Implementers).
pub fn execute(
    jules_path: &Path,
    layer: Layer,
    issue_path: &Path,
    dry_run: bool,
    branch: Option<&str>,
    is_ci: bool,
) -> Result<RunResult, AppError> {
    // Validate issue file requirement (now guaranteed by Clap)
    let path = issue_path;

    if !path.exists() {
        return Err(AppError::IssueFileNotFound(path.display().to_string()));
    }

    // Security Check: Ensure path is within .jules/workstreams/*/issues
    let canonical_path = fs::canonicalize(path)?;
    let abs_jules_path = if jules_path.is_absolute() {
        jules_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(jules_path)
    };
    let workstreams_dir = fs::canonicalize(abs_jules_path.join("workstreams"))
        .map_err(|_| AppError::Configuration("Workstreams directory not found".into()))?;

    let has_issues_component = canonical_path.components().any(|c| c.as_os_str() == "issues");
    if !canonical_path.starts_with(&workstreams_dir) || !has_issues_component {
        return Err(AppError::Configuration(format!(
            "Issue file must be within {}/*/issues/",
            workstreams_dir.display()
        )));
    }

    // Handle Local Dispatch (outside CI)
    if !is_ci {
        return execute_local_dispatch(&canonical_path, layer, dry_run);
    }

    // CI Execution: Direct session creation
    let issue_content = fs::read_to_string(path)?;
    let config = load_config(jules_path)?;

    // Determine starting branch
    let starting_branch = branch.map(String::from).unwrap_or_else(|| {
        if layer == Layer::Implementers {
            config.run.default_branch.clone()
        } else {
            // Planners work on the jules branch
            config.run.jules_branch.clone()
        }
    });

    if dry_run {
        execute_dry_run(jules_path, layer, &starting_branch, &issue_content, path)?;
        return Ok(RunResult {
            roles: vec![layer.dir_name().to_string()],
            dry_run: true,
            sessions: vec![],
        });
    }

    // Determine repository source from git
    let source = detect_repository_source()?;

    // Execute with appropriate client
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = execute_session(
        jules_path,
        layer,
        &starting_branch,
        &source,
        &client,
        &issue_content,
        path,
    )?;

    Ok(RunResult {
        roles: vec![layer.dir_name().to_string()],
        dry_run: false,
        sessions: vec![session_id],
    })
}

/// Execute local workflow dispatch via gh CLI.
fn execute_local_dispatch(
    canonical_path: &Path,
    layer: Layer,
    dry_run: bool,
) -> Result<RunResult, AppError> {
    let workflow_name = match layer {
        Layer::Planners => PLANNER_WORKFLOW_NAME,
        Layer::Implementers => IMPLEMENTER_WORKFLOW_NAME,
        _ => unreachable!("Single-role check already done"),
    };

    if dry_run {
        println!("=== Dry Run: Local Dispatch ===");
        println!("Would dispatch workflow '{}' for: {}", workflow_name, canonical_path.display());
        return Ok(RunResult { roles: vec![], dry_run: true, sessions: vec![] });
    }

    println!(
        "Dispatching {} workflow for: {}",
        layer.display_name().to_lowercase(),
        canonical_path.display()
    );

    // Compute relative path for workflow input
    let current_dir = std::env::current_dir()?;
    let relative_path = canonical_path.strip_prefix(&current_dir).unwrap_or(canonical_path);

    // Execute: gh workflow run <workflow> -f issue_file=<path>
    let output = std::process::Command::new("gh")
        .args([
            "workflow",
            "run",
            workflow_name,
            "-f",
            &format!("issue_file={}", relative_path.display()),
        ])
        .output()
        .map_err(|e| AppError::Configuration(format!("Failed to execute gh CLI: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Configuration(format!(
            "Failed to dispatch workflow via gh CLI. Stderr:\n{}",
            stderr
        )));
    }

    println!("✅ Workflow dispatched successfully.");

    let role_name = format!("{}-dispatch", layer.dir_name().trim_end_matches('s'));
    Ok(RunResult { roles: vec![role_name], dry_run: false, sessions: vec![] })
}

/// Execute a single role with the given Jules client.
fn execute_session<C: JulesClient>(
    jules_path: &Path,
    layer: Layer,
    starting_branch: &str,
    source: &str,
    client: &C,
    issue_content: &str,
    issue_path: &Path,
) -> Result<String, AppError> {
    println!("Executing {}...", layer.display_name());

    let mut prompt = assemble_single_role_prompt(jules_path, layer)?;

    // Append issue content
    prompt.push_str("\n---\n# Issue Content\n");
    if layer == Layer::Planners {
        prompt.push_str(&format!("File: {}\n\n", issue_path.display()));
    }
    prompt.push_str(issue_content);

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch: starting_branch.to_string(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(response.session_id)
}

/// Execute a dry run for a single-role layer.
fn execute_dry_run(
    jules_path: &Path,
    layer: Layer,
    starting_branch: &str,
    issue_content: &str,
    issue_path: &Path,
) -> Result<(), AppError> {
    println!("=== Dry Run: {} ===", layer.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Issue content: {} chars\n", issue_content.len());

    let layer_dir = jules_path.join("roles").join(layer.dir_name());
    let prompt_path = layer_dir.join("prompt.yml");
    let contracts_path = layer_dir.join("contracts.yml");

    println!("Prompt: {}", prompt_path.display());
    if contracts_path.exists() {
        println!("Contracts: {}", contracts_path.display());
    }

    if let Ok(mut prompt) = assemble_single_role_prompt(jules_path, layer) {
        prompt.push_str("\n---\n# Issue Content\n");
        if layer == Layer::Planners {
            prompt.push_str(&format!("File: {}\n\n", issue_path.display()));
        }
        prompt.push_str(issue_content);

        println!(
            "Assembled prompt: {} chars (Prompt + {} + Issue Content)",
            prompt.len(),
            if layer == Layer::Planners { "Issue Path" } else { "No Path" }
        );
    }

    println!("\nWould execute 1 session");
    Ok(())
}
