use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::adapters::catalogs::workflow_scaffold::load_workflow_scaffold;
use crate::domain::config::WorkflowGenerateConfig;
use crate::domain::{AppError, JLO_DIR, WorkflowRunnerMode};
use crate::ports::Git;

#[derive(Debug, Default)]
pub struct DeinitOutcome {
    pub deleted_branch: bool,
    pub deleted_files: Vec<String>,
    pub deleted_action_dirs: Vec<String>,
    pub deleted_jlo: bool,
}

pub fn execute(root: &Path, git: &impl Git) -> Result<DeinitOutcome, AppError> {
    let current_branch = git.get_current_branch()?;
    if current_branch == "jules" || current_branch.starts_with("jules-test-") {
        return Err(AppError::Validation(format!(
            "Cannot deinit while on branch '{}'. Switch to your main branch and re-run.",
            current_branch
        )));
    }

    let mut file_paths = BTreeSet::new();
    let mut action_dirs = BTreeSet::new();

    let generate_config = WorkflowGenerateConfig::default();
    for mode in [WorkflowRunnerMode::remote(), WorkflowRunnerMode::self_hosted()] {
        let scaffold = load_workflow_scaffold(&mode, &generate_config)?;
        for file in scaffold.files {
            file_paths.insert(file.path);
        }
        for action_dir in scaffold.action_dirs {
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

    // Remove .jlo/ control plane
    let jlo_path = root.join(JLO_DIR);
    let deleted_jlo = if jlo_path.exists() {
        fs::remove_dir_all(&jlo_path)?;
        true
    } else {
        false
    };

    Ok(DeinitOutcome { deleted_branch, deleted_files, deleted_action_dirs, deleted_jlo })
}
