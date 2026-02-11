use std::path::{Path, PathBuf};

use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer};
use crate::ports::{GitHubPort, WorkspaceStore};

use super::RunResult;

const PLANNER_WORKFLOW_NAME: &str = "jules-run-planner.yml";
const IMPLEMENTER_WORKFLOW_NAME: &str = "jules-run-implementer.yml";

pub(crate) struct IssuePathInfo {
    pub(crate) issue_path_str: String,
    pub(crate) canonical_path: PathBuf,
}

pub(crate) fn validate_issue_path<W: WorkspaceStore>(
    issue_path: &Path,
    workspace: &W,
) -> Result<IssuePathInfo, AppError> {
    let path_str = issue_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Issue path contains invalid unicode".to_string()))?;

    if !issue_path.exists() {
        return Err(AppError::IssueFileNotFound(path_str.to_string()));
    }

    let canonical_path = workspace.canonicalize(path_str)?;

    let exchange_dir = jules::exchange_dir(&workspace.jules_path());
    let exchange_dir_str = exchange_dir.to_str().ok_or_else(|| {
        AppError::Validation("Exchange path contains invalid unicode".to_string())
    })?;

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

    Ok(IssuePathInfo { issue_path_str: path_str.to_string(), canonical_path })
}

pub(crate) fn execute_local_dispatch<H, W>(
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
        Layer::Planner => PLANNER_WORKFLOW_NAME,
        Layer::Implementer => IMPLEMENTER_WORKFLOW_NAME,
        _ => unreachable!("Issue-driven check already done"),
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

    let root = workspace.resolve_path("");
    let canonical_root = workspace.canonicalize("").unwrap_or(root);
    let relative_path = canonical_path.strip_prefix(&canonical_root).unwrap_or(canonical_path);

    let inputs = &[("issue_file", relative_path.to_str().unwrap_or(""))];

    github.dispatch_workflow(workflow_name, inputs)?;

    println!("✅ Workflow dispatched successfully.");

    let role_name = format!("{}-dispatch", layer.dir_name());
    Ok(RunResult { roles: vec![role_name], prompt_preview: false, sessions: vec![] })
}
