use std::collections::HashSet;

use crate::adapters::assets::workflow_scaffold_assets::{
    WorkflowGenerateConfig, load_workflow_scaffold, WorkflowScaffoldAssets,
};
use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::WorkspaceStore;

/// Execute the workflow scaffold installation.
pub fn install_workflow_scaffold(
    workspace: &impl WorkspaceStore,
    mode: &WorkflowRunnerMode,
    generate_config: &WorkflowGenerateConfig,
) -> Result<(), AppError> {
    let scaffold = load_workflow_scaffold(mode, generate_config)?;
    remove_stale_managed_workflows(workspace, &scaffold)?;

    for action_dir in &scaffold.action_dirs {
        if workspace.is_dir(action_dir) {
            workspace.remove_dir_all(action_dir)?;
        } else if workspace.file_exists(action_dir) {
            workspace.remove_file(action_dir)?;
        }
    }

    for file in &scaffold.files {
        workspace.write_file(&file.path, &file.content)?;
    }

    Ok(())
}

fn remove_stale_managed_workflows(
    workspace: &impl WorkspaceStore,
    scaffold: &WorkflowScaffoldAssets,
) -> Result<(), AppError> {
    let workflows_dir = ".github/workflows";
    // Check if workflows dir exists (either as file or dir, though should be dir)
    if !workspace.file_exists(workflows_dir) && !workspace.is_dir(workflows_dir) {
        return Ok(());
    }

    // rendered_paths are relative paths in scaffold.files (e.g. .github/workflows/foo.yml)
    // We construct absolute paths for comparison with list_dir results.
    let rendered_paths: HashSet<_> =
        scaffold.files.iter().map(|file| workspace.resolve_path(&file.path)).collect();

    let entries = workspace.list_dir(workflows_dir)?;
    for path in entries {
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };

        if !path.is_file() {
            continue;
        }

        let is_yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "yml" || ext == "yaml");

        if !is_yaml || !file_name.starts_with("jules-") {
            continue;
        }

        if rendered_paths.contains(&path) {
            continue;
        }

        // Remove file. Passing absolute path string works with WorkspaceStore implementations.
        if let Some(path_str) = path.to_str() {
             workspace.remove_file(path_str)?;
        }
    }

    Ok(())
}
