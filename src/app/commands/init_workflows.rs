use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::services::workflow_kit_assets::load_workflow_kit;

/// Execute the workflow kit installation.
pub fn execute_workflows(
    root: &Path,
    mode: WorkflowRunnerMode,
    overwrite: bool,
) -> Result<(), AppError> {
    let kit = load_workflow_kit(mode)?;

    let mut collisions = BTreeSet::new();
    for file in &kit.files {
        let destination = root.join(&file.path);
        if destination.exists() {
            collisions.insert(file.path.clone());
        }
    }

    for action_dir in &kit.action_dirs {
        let destination = root.join(action_dir);
        if destination.exists() {
            collisions.insert(action_dir.clone());
        }
    }

    if !overwrite && !collisions.is_empty() {
        let mut message = format!(
            "Workflow kit install aborted (mode: {}).\nThe following paths already exist:\n",
            mode.label()
        );
        for path in collisions {
            message.push_str(&format!("  - {}\n", path));
        }
        message.push_str("Re-run with --overwrite to replace kit-owned files.");
        return Err(AppError::config_error(message));
    }

    if overwrite {
        for action_dir in &kit.action_dirs {
            let destination = root.join(action_dir);
            if destination.exists() {
                if destination.is_dir() {
                    fs::remove_dir_all(&destination)?;
                } else {
                    fs::remove_file(&destination)?;
                }
            }
        }
    }

    for file in &kit.files {
        let destination = root.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, &file.content)?;
    }

    Ok(())
}
