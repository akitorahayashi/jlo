use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_yaml::Value;

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::services::workflow_kit_assets::load_workflow_kit;

const WORKFLOW_PATH: &str = ".github/workflows/jules-workflows.yml";

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
        let mut content = file.content.clone();
        if overwrite
            && destination.exists()
            && file.path == WORKFLOW_PATH
            && let Some(schedule) = load_existing_schedule(&destination)?
        {
            content = replace_schedule_block(&content, &schedule)?;
        }
        fs::write(&destination, content)?;
    }

    Ok(())
}

fn load_existing_schedule(path: &Path) -> Result<Option<Value>, AppError> {
    let content = fs::read_to_string(path)?;
    let value: Value = serde_yaml::from_str(&content).map_err(|err| {
        AppError::config_error(format!(
            "Failed to parse existing workflow YAML for schedule preservation at {}: {}",
            path.display(),
            err
        ))
    })?;

    extract_schedule(value, path)
}

fn extract_schedule(value: Value, path: &Path) -> Result<Option<Value>, AppError> {
    let mapping = match value {
        Value::Mapping(map) => map,
        _ => {
            return Err(AppError::config_error(format!(
                "Existing workflow YAML must be a mapping for schedule preservation at {}",
                path.display()
            )));
        }
    };

    let on_key = Value::String("on".to_string());
    let Some(on_value) = mapping.get(&on_key) else {
        return Ok(None);
    };

    let Value::Mapping(on_map) = on_value else {
        return Ok(None);
    };

    let schedule_key = Value::String("schedule".to_string());
    let Some(schedule_value) = on_map.get(&schedule_key) else {
        return Ok(None);
    };

    match schedule_value {
        Value::Sequence(_) => Ok(Some(schedule_value.clone())),
        _ => Err(AppError::config_error(format!(
            "Existing workflow schedule must be a YAML sequence at {}",
            path.display()
        ))),
    }
}

fn replace_schedule_block(template: &str, schedule: &Value) -> Result<String, AppError> {
    let schedule_yaml = serde_yaml::to_string(schedule).map_err(|err| {
        AppError::config_error(format!("Failed to render existing workflow schedule: {}", err))
    })?;

    let schedule_lines: Vec<String> =
        schedule_yaml.lines().map(|line| format!("    {}", line)).collect();

    let mut output = Vec::new();
    let mut lines = template.lines().peekable();
    let mut replaced = false;

    while let Some(line) = lines.next() {
        if !replaced && line.trim_end() == "  schedule:" {
            output.push("  schedule:".to_string());
            output.extend(schedule_lines.iter().cloned());

            while let Some(peek) = lines.peek() {
                let indent = peek.chars().take_while(|c| *c == ' ').count();
                if indent > 2 {
                    lines.next();
                    continue;
                }
                break;
            }

            replaced = true;
            continue;
        }

        output.push(line.to_string());
    }

    if !replaced {
        return Err(AppError::config_error(
            "Workflow kit missing on.schedule block for preservation",
        ));
    }

    let mut result = output.join("\n");
    if template.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}
