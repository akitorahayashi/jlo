use std::path::Path;

use crate::adapters::jules_client_http::HttpJulesClient;
use crate::app::commands::workflow::{WorkflowExchangeCleanIssueOptions, clean_issue};
use crate::domain::prompt_assembly::implementer as implementer_asm;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer};
use crate::ports::{AutomationMode, JulesClient, SessionRequest, WorkspaceStore};

use super::RunOptions;
use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::issue_execution::validate_issue_path;

/// Execute the implementer layer (single-role, issue-driven).
pub(crate) fn execute<W>(
    jules_path: &Path,
    options: &RunOptions,
    issue_path: &Path,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let issue_info = validate_issue_path(issue_path, workspace)?;

    let issue_content = workspace.read_file(&issue_info.issue_path_str)?;
    let config = load_config(jules_path)?;

    let starting_branch =
        options.branch.clone().unwrap_or_else(|| config.run.default_branch.clone());

    if options.prompt_preview {
        execute_prompt_preview(jules_path, &starting_branch, &issue_content, workspace)?;
        return Ok(RunResult {
            roles: vec![Layer::Implementer.dir_name().to_string()],
            prompt_preview: true,
            sessions: vec![],
        });
    }

    let source = detect_repository_source()?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let session_id =
        execute_session(jules_path, &starting_branch, &source, &client, &issue_content, workspace)?;

    let cleanup_output = clean_issue(WorkflowExchangeCleanIssueOptions {
        issue_file: issue_info.issue_path_str.clone(),
    })?;
    println!(
        "✅ Cleaned issue and source events ({} file(s) removed)",
        cleanup_output.deleted_paths.len()
    );

    Ok(RunResult {
        roles: vec![Layer::Implementer.dir_name().to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_session<C: JulesClient, W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    starting_branch: &str,
    source: &str,
    client: &C,
    issue_content: &str,
    workspace: &W,
) -> Result<String, AppError> {
    println!("Executing {}...", Layer::Implementer.display_name());

    let mut prompt = assemble_implementer_prompt(jules_path, issue_content, workspace)?;

    prompt.push_str("\n---\n# Issue Content\n");
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

fn assemble_implementer_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    issue_content: &str,
    workspace: &W,
) -> Result<String, AppError> {
    let label = extract_issue_label(issue_content)?;
    let task_content = resolve_implementer_task(jules_path, &label, workspace)?;
    let input = implementer_asm::ImplementerPromptInput { task: &task_content };
    let assembled = implementer_asm::assemble(jules_path, &input, workspace)?;
    Ok(assembled.content)
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

    if !crate::domain::identifiers::validation::validate_safe_path_component(label) {
        return Err(AppError::Validation(format!(
            "Invalid label '{}': must be a safe path component",
            label
        )));
    }

    Ok(label.to_string())
}

/// Resolve the label-specific task file for implementer.
///
/// Maps label to `tasks/<label>.yml`. Fails if the task file does not exist.
fn resolve_implementer_task<W: WorkspaceStore>(
    jules_path: &Path,
    label: &str,
    workspace: &W,
) -> Result<String, AppError> {
    let task_path = jules::tasks_dir(jules_path, Layer::Implementer).join(format!("{}.yml", label));

    workspace.read_file(&task_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "No task file for label '{}': expected {}",
            label,
            task_path.display()
        ))
    })
}

fn execute_prompt_preview<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    starting_branch: &str,
    issue_content: &str,
    workspace: &W,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: {} ===", Layer::Implementer.display_name());
    println!("Starting branch: {}\n", starting_branch);
    println!("Issue content: {} chars\n", issue_content.len());

    let prompt_path = jules::prompt_template(jules_path, Layer::Implementer);
    let contracts_path = jules::contracts(jules_path, Layer::Implementer);

    println!("Prompt: {}", prompt_path.display());
    if contracts_path.exists() {
        println!("Contracts: {}", contracts_path.display());
    }

    if let Ok(mut prompt) = assemble_implementer_prompt(jules_path, issue_content, workspace) {
        prompt.push_str("\n---\n# Issue Content\n");
        prompt.push_str(issue_content);

        println!("Assembled prompt: {} chars (Prompt + No Path + Issue Content)", prompt.len());
    }

    println!("\nWould execute 1 session");
    Ok(())
}
