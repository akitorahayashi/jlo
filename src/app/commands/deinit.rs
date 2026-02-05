use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::GitPort;
use crate::services::assets::workflow_kit_assets::load_workflow_kit;

#[derive(Debug, Default)]
pub struct DeinitOutcome {
    pub deleted_branch: bool,
    pub deleted_files: Vec<String>,
    pub deleted_action_dirs: Vec<String>,
}

pub fn execute(root: &Path, git: &impl GitPort) -> Result<DeinitOutcome, AppError> {
    let current_branch = git.get_current_branch()?;
    if current_branch == "jules" || current_branch.starts_with("jules-test-") {
        return Err(AppError::Validation { reason: format!(
            "Cannot deinit while on branch '{}'. Switch to your main branch and re-run.",
            current_branch
        ) });
    }

    let mut file_paths = BTreeSet::new();
    let mut action_dirs = BTreeSet::new();

    for mode in [WorkflowRunnerMode::Remote, WorkflowRunnerMode::SelfHosted] {
        let kit = load_workflow_kit(mode)?;
        for file in kit.files {
            file_paths.insert(file.path);
        }
        for action_dir in kit.action_dirs {
            action_dirs.insert(action_dir);
        }
    }

    let mut deleted_files = Vec::new();
    for path in &file_paths {
        let target = root.join(path);
        if target.exists() {
            fs::remove_file(&target)?;
            deleted_files.push(path.clone());
        }
    }

    let mut deleted_action_dirs = Vec::new();
    for dir in &action_dirs {
        let target = root.join(dir);
        if target.exists() {
            fs::remove_dir_all(&target)?;
            deleted_action_dirs.push(dir.clone());
        }
    }

    let deleted_branch = git.delete_branch("jules", true)?;

    Ok(DeinitOutcome { deleted_branch, deleted_files, deleted_action_dirs })
}
