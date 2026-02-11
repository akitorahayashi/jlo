//! Single-role layer execution (Planners, Implementers).

use std::path::Path;

use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::prompt::assemble_single_role_prompt;
use crate::adapters::jules_client_http::HttpJulesClient;
use crate::domain::prompt_assembly::implementer as implementer_asm;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer};
use crate::ports::{AutomationMode, GitHubPort, JulesClient, SessionRequest, WorkspaceStore};

const PLANNER_WORKFLOW_NAME: &str = "jules-run-planner.yml";
const IMPLEMENTER_WORKFLOW_NAME: &str = "jules-run-implementer.yml";

/// Execute a single-role layer (Planners or Implementers).
#[allow(clippy::too_many_arguments)]
pub fn execute<H, W>(
    jules_path: &Path,
    layer: Layer,
    issue_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    is_ci: bool,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    H: GitHubPort,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    // Validate issue file requirement
    let path_str = issue_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Issue path contains invalid unicode".to_string()))?;

    if !issue_path.exists() {
        return Err(AppError::IssueFileNotFound(path_str.to_string()));
    }

    // Security Check: Ensure path is within .jules/exchange/*/issues
    // We use workspace.canonicalize to resolve absolute path
    let canonical_path = workspace.canonicalize(path_str)?;

    // We expect exchange to be in .jules/exchange relative to workspace root
    // workspace.jules_path() returns the .jules directory path
    let exchange_dir = jules::exchange_dir(&workspace.jules_path());
    // Canonicalize exchange dir to compare apples to apples (resolve potential symlinks)
    // We use workspace.canonicalize on the string representation
    let exchange_dir_str = exchange_dir.to_str().ok_or_else(|| {
        AppError::Validation("Exchange path contains invalid unicode".to_string())
    })?;

    // Note: canonicalize might fail if dir doesn't exist, but it should exist if workspace is valid
    let canonical_exchange_dir = workspace
        .canonicalize(exchange_dir_str)
        .map_err(|_| AppError::ExchangeDirectoryNotFound)?;

    let has_issues_component = canonical_path.components().any(|c| c.as_os_str() == "issues");
    if !canonical_path.starts_with(&canonical_exchange_dir) || !has_issues_component {
        return Err(AppError::Validation(format!(
            "Issue file must be within {}/*/issues/",
            canonical_exchange_dir.display()
        )));
    }

    // Handle Local Dispatch (outside CI)
    if !is_ci {
        return execute_local_dispatch(&canonical_path, layer, prompt_preview, github, workspace);
    }

    // CI Execution: Direct session creation
    let issue_content = workspace.read_file(path_str)?;
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

    if prompt_preview {
        execute_prompt_preview(
            jules_path,
            layer,
            &starting_branch,
            &issue_content,
            issue_path,
            workspace,
        )?;
        return Ok(RunResult {
            roles: vec![layer.dir_name().to_string()],
            prompt_preview: true,
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
        issue_path,
        workspace,
    )?;

    Ok(RunResult {
        roles: vec![layer.dir_name().to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
    })
}

/// Execute local workflow dispatch via GitHubPort.
fn execute_local_dispatch<H, W>(
    canonical_path: &Path,
    layer: Layer,
    prompt_preview: bool,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    H: GitHubPort,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let workflow_name = match layer {
        Layer::Planners => PLANNER_WORKFLOW_NAME,
        Layer::Implementers => IMPLEMENTER_WORKFLOW_NAME,
        _ => unreachable!("Single-role check already done"),
    };

    if prompt_preview {
        println!("=== Prompt Preview: Local Dispatch ===");
        println!("Would dispatch workflow '{}' for: {}", workflow_name, canonical_path.display());
        return Ok(RunResult { roles: vec![], prompt_preview: true, sessions: vec![] });
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
    Ok(RunResult { roles: vec![role_name], prompt_preview: false, sessions: vec![] })
}

/// Execute a single role with the given Jules client.
#[allow(clippy::too_many_arguments)]
fn execute_session<C: JulesClient, W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    layer: Layer,
    starting_branch: &str,
    source: &str,
    client: &C,
    issue_content: &str,
    issue_path: &Path,
    workspace: &W,
) -> Result<String, AppError> {
    println!("Executing {}...", layer.display_name());

    let mut prompt = assemble_layer_prompt(jules_path, layer, issue_content, workspace)?;

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

/// Assemble the prompt for a single-role layer, dispatching to the correct assembler.
///
/// Implementers use their label-specific assembler; planners use the generic
/// single-role assembly.
fn assemble_layer_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    layer: Layer,
    issue_content: &str,
    workspace: &W,
) -> Result<String, AppError> {
    if layer == Layer::Implementers {
        let label = extract_issue_label(issue_content)?;
        let task_content = resolve_implementer_task(jules_path, &label, workspace)?;
        let input = implementer_asm::ImplementerPromptInput { task: &task_content };
        let assembled = implementer_asm::assemble(jules_path, &input, workspace)?;
        return Ok(assembled.content);
    }

    assemble_single_role_prompt(jules_path, layer, workspace)
}

/// Extract the `label` field from issue YAML content.
///
/// Fails explicitly if the label is missing, empty, or unsafe — no silent fallback.
fn extract_issue_label(issue_content: &str) -> Result<String, AppError> {
    let value: serde_yaml::Value = serde_yaml::from_str(issue_content)
        .map_err(|e| AppError::Validation(format!("Failed to parse issue YAML: {}", e)))?;

    let label =
        value.get("label").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or_else(|| {
            AppError::Validation("Issue file must contain a non-empty 'label' field".to_string())
        })?;

    // Prevent path traversal via crafted label values
    if !crate::domain::identifiers::validation::validate_safe_path_component(label) {
        return Err(AppError::Validation(format!(
            "Invalid label '{}': must be a safe path component",
            label
        )));
    }

    Ok(label.to_string())
}

/// Resolve the label-specific task file for implementers.
///
/// Maps label to `tasks/<label>.yml`. Fails if the task file does not exist.
fn resolve_implementer_task<W: WorkspaceStore>(
    jules_path: &Path,
    label: &str,
    workspace: &W,
) -> Result<String, AppError> {
    let task_path =
        jules::tasks_dir(jules_path, Layer::Implementers).join(format!("{}.yml", label));

    workspace.read_file(&task_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "No task file for label '{}': expected {}",
            label,
            task_path.display()
        ))
    })
}

/// Execute a prompt preview for a single-role layer.
fn execute_prompt_preview<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    layer: Layer,
    starting_branch: &str,
    issue_content: &str,
    issue_path: &Path,
    workspace: &W,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", layer.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Issue content: {} chars\n", issue_content.len());

    let prompt_path = jules::prompt_template(jules_path, layer);
    let contracts_path = jules::contracts(jules_path, layer);

    println!("Prompt: {}", prompt_path.display());
    if contracts_path.exists() {
        println!("Contracts: {}", contracts_path.display());
    }

    if let Ok(mut prompt) = assemble_layer_prompt(jules_path, layer, issue_content, workspace) {
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
