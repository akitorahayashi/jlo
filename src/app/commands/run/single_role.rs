//! Single-role layer execution (Planners, Implementers).

use std::path::Path;

use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::prompt::assemble_single_role_prompt;
use crate::domain::{AppError, Layer, JULES_DIR};
use crate::ports::{AutomationMode, GitHubPort, JulesClient, SessionRequest, WorkspaceStore};
use crate::services::adapters::jules_client_http::HttpJulesClient;

const PLANNER_WORKFLOW_NAME: &str = "jules-run-planner.yml";
const IMPLEMENTER_WORKFLOW_NAME: &str = "jules-run-implementer.yml";

/// Execute a single-role layer (Planners or Implementers).
#[allow(clippy::too_many_arguments)]
pub fn execute<H, W>(
    _jules_path: &Path,
    layer: Layer,
    issue_path: &Path,
    dry_run: bool,
    branch: Option<&str>,
    is_ci: bool,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    H: GitHubPort,
    W: WorkspaceStore,
{
    // Validate issue file requirement
    let path_str = issue_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Issue path contains invalid unicode".to_string()))?;

    if !workspace.path_exists(path_str) {
        return Err(AppError::IssueFileNotFound(path_str.to_string()));
    }

    // Security Check: Ensure path is within .jules/workstreams/*/issues
    // We use workspace.canonicalize to resolve absolute path
    let canonical_path = workspace.canonicalize(path_str)?;

    // We expect workstreams to be in .jules/workstreams relative to workspace root
    // workspace.jules_path() returns the .jules directory path
    let workstreams_dir = workspace.jules_path().join("workstreams");
    // Canonicalize workstreams dir to compare apples to apples (resolve potential symlinks)
    // We use workspace.canonicalize on the string representation
    let workstreams_dir_str = workstreams_dir.to_str().ok_or_else(|| {
        AppError::Validation("Workstreams path contains invalid unicode".to_string())
    })?;

    // Note: canonicalize might fail if dir doesn't exist, but it should exist if workspace is valid
    let canonical_workstreams_dir = workspace
        .canonicalize(workstreams_dir_str)
        .map_err(|_| AppError::WorkstreamsDirectoryNotFound)?;

    let has_issues_component = canonical_path.components().any(|c| c.as_os_str() == "issues");
    if !canonical_path.starts_with(&canonical_workstreams_dir) || !has_issues_component {
        return Err(AppError::Validation(format!(
            "Issue file must be within {}/*/issues/",
            canonical_workstreams_dir.display()
        )));
    }

    // Handle Local Dispatch (outside CI)
    if !is_ci {
        return execute_local_dispatch(&canonical_path, layer, dry_run, github, workspace);
    }

    // CI Execution: Direct session creation
    let issue_content = workspace.read_file(path_str)?;
    let config = load_config(workspace)?;

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
        execute_dry_run(workspace, layer, &starting_branch, &issue_content, issue_path)?;
        return Ok(RunResult {
            roles: vec![layer.dir_name().to_string()],
            dry_run: true,
            sessions: vec![],
        });
    }

    // Determine repository source from git
    // Note: detect_repository_source currently uses direct git command/config check.
    // It should also be refactored eventually, but it's in `config.rs`.
    // For now we leave it as is, or we should use GitPort?
    // The task didn't explicitly mention config.rs but "Application commands".
    // I will leave detect_repository_source as is for now as it wasn't listed in affected areas,
    // though ideally it should be refactored too.
    let source = detect_repository_source()?;

    // Execute with appropriate client
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id = execute_session(
        workspace,
        layer,
        &starting_branch,
        &source,
        &client,
        &issue_content,
        issue_path,
    )?;

    Ok(RunResult {
        roles: vec![layer.dir_name().to_string()],
        dry_run: false,
        sessions: vec![session_id],
    })
}

/// Execute local workflow dispatch via GitHubPort.
fn execute_local_dispatch<H, W>(
    canonical_path: &Path,
    layer: Layer,
    dry_run: bool,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    H: GitHubPort,
    W: WorkspaceStore,
{
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
    // We resolve relative to workspace root
    let root = workspace.resolve_path("");
    // If canonical_path (absolute) starts with root (absolute), strip prefix.
    // Note: resolve_path returns absolute path if root was absolute (which it is from current_dir).
    // But we need to make sure 'root' is also canonicalized to match canonical_path?
    // FilesystemWorkspaceStore::new stores root as is.
    // We should canonicalize root.
    let canonical_root = workspace.canonicalize("").unwrap_or(root);

    let relative_path = canonical_path.strip_prefix(&canonical_root).unwrap_or(canonical_path);

    // Execute via port
    let inputs = &[("issue_file", relative_path.to_str().unwrap_or(""))];

    github.dispatch_workflow(workflow_name, inputs)?;

    println!("✅ Workflow dispatched successfully.");

    let role_name = format!("{}-dispatch", layer.dir_name().trim_end_matches('s'));
    Ok(RunResult { roles: vec![role_name], dry_run: false, sessions: vec![] })
}

/// Execute a single role with the given Jules client.
fn execute_session<C: JulesClient>(
    workspace: &impl WorkspaceStore,
    layer: Layer,
    starting_branch: &str,
    source: &str,
    client: &C,
    issue_content: &str,
    issue_path: &Path,
) -> Result<String, AppError> {
    println!("Executing {}...", layer.display_name());

    let mut prompt = assemble_single_role_prompt(workspace, layer)?;

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
    workspace: &impl WorkspaceStore,
    layer: Layer,
    starting_branch: &str,
    issue_content: &str,
    issue_path: &Path,
) -> Result<(), AppError> {
    println!("=== Dry Run: {} ===", layer.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Issue content: {} chars\n", issue_content.len());

    let prompt_path = format!("{}/roles/{}/prompt.yml", JULES_DIR, layer.dir_name());
    let contracts_path = format!("{}/roles/{}/contracts.yml", JULES_DIR, layer.dir_name());

    println!("Prompt: {}", prompt_path);
    if workspace.path_exists(&contracts_path) {
        println!("Contracts: {}", contracts_path);
    }

    if let Ok(mut prompt) = assemble_single_role_prompt(workspace, layer) {
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
