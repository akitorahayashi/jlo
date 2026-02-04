use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_yaml::Value;

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::services::assets::workflow_kit_assets::load_workflow_kit;

/// The workflow file whose schedule should be preserved during overwrite.
const SCHEDULE_PRESERVE_FILE: &str = ".github/workflows/jules-workflows.yml";

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

    // Extract existing schedule if overwriting the main workflow file
    let preserved_schedule = if overwrite { extract_existing_schedule(root)? } else { None };

    for file in &kit.files {
        let destination = root.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        // Merge preserved schedule into the main workflow file
        let content = if file.path == SCHEDULE_PRESERVE_FILE {
            if let Some(ref schedule) = preserved_schedule {
                merge_schedule_into_workflow(&file.content, schedule)?
            } else {
                file.content.clone()
            }
        } else {
            file.content.clone()
        };

        fs::write(&destination, content)?;
    }

    Ok(())
}

/// Extract the `on.schedule` block from the existing workflow file.
/// Returns `None` if the file doesn't exist or has no schedule.
/// Returns an error if the file exists but cannot be parsed.
fn extract_existing_schedule(root: &Path) -> Result<Option<Value>, AppError> {
    let workflow_path = root.join(SCHEDULE_PRESERVE_FILE);
    if !workflow_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&workflow_path)?;
    let yaml: Value = serde_yaml::from_str(&content).map_err(|e| AppError::ParseError {
        what: format!("existing workflow file '{}'", SCHEDULE_PRESERVE_FILE),
        details: e.to_string(),
    })?;

    let schedule = yaml.get("on").and_then(|on| on.get("schedule")).cloned();

    Ok(schedule)
}

/// Merge a preserved schedule into the kit workflow content.
fn merge_schedule_into_workflow(kit_content: &str, schedule: &Value) -> Result<String, AppError> {
    let mut yaml: Value = serde_yaml::from_str(kit_content).map_err(|e| AppError::ParseError {
        what: "workflow kit content".to_string(),
        details: e.to_string(),
    })?;

    if let Value::Mapping(root) = &mut yaml
        && let Some(Value::Mapping(on_block)) = root.get_mut("on")
    {
        on_block.insert(Value::String("schedule".to_string()), schedule.clone());
    }

    serde_yaml::to_string(&yaml).map_err(|e| AppError::ParseError {
        what: "merged workflow content".to_string(),
        details: e.to_string(),
    })
}
