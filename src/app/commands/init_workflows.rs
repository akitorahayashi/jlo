use std::fs;
use std::path::Path;

use serde_yaml::Value;

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::adapters::assets::workflow_kit_assets::load_workflow_kit;

/// The workflow file whose schedule should be preserved during overwrite.
const SCHEDULE_PRESERVE_FILE: &str = ".github/workflows/jules-workflows.yml";

/// Execute the workflow kit installation.
pub fn execute_workflows(root: &Path, mode: WorkflowRunnerMode) -> Result<(), AppError> {
    let kit = load_workflow_kit(mode)?;

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

    // Extract existing configuration from the main workflow file before overwrite
    let preserved_config = extract_preserved_config(root)?;

    for file in &kit.files {
        let destination = root.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        // Merge preserved configuration into the main workflow file
        let content = if file.path == SCHEDULE_PRESERVE_FILE {
            if preserved_config.has_values() {
                merge_config_into_workflow(&file.content, &preserved_config)?
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

#[derive(Debug, Default)]
struct PreservedConfig {
    schedule: Option<Value>,
    wait_minutes_default: Option<Value>,
}

impl PreservedConfig {
    fn has_values(&self) -> bool {
        self.schedule.is_some() || self.wait_minutes_default.is_some()
    }
}

/// Extract preserved configuration (schedule, wait_minutes) from the existing workflow file.
fn extract_preserved_config(root: &Path) -> Result<PreservedConfig, AppError> {
    let workflow_path = root.join(SCHEDULE_PRESERVE_FILE);
    if !workflow_path.exists() {
        return Ok(PreservedConfig::default());
    }

    let content = fs::read_to_string(&workflow_path)?;
    let yaml: Value = serde_yaml::from_str(&content).map_err(|e| AppError::ParseError {
        what: format!("existing workflow file '{}'", SCHEDULE_PRESERVE_FILE),
        details: e.to_string(),
    })?;

    let schedule = yaml.get("on").and_then(|on| on.get("schedule")).cloned();

    let wait_minutes_default = yaml
        .get("on")
        .and_then(|on| on.get("workflow_dispatch"))
        .and_then(|wd| wd.get("inputs"))
        .and_then(|inputs| inputs.get("wait_minutes"))
        .and_then(|wm| wm.get("default"))
        .cloned();

    Ok(PreservedConfig { schedule, wait_minutes_default })
}

/// Merge preserved configuration into the kit workflow content.
fn merge_config_into_workflow(
    kit_content: &str,
    config: &PreservedConfig,
) -> Result<String, AppError> {
    let mut yaml: Value = serde_yaml::from_str(kit_content).map_err(|e| AppError::ParseError {
        what: "workflow kit content".to_string(),
        details: e.to_string(),
    })?;

    let root = yaml.as_mapping_mut().ok_or_else(|| {
        AppError::Validation(
            "Could not preserve config: workflow kit root is not a mapping.".into(),
        )
    })?;

    if let Some(ref schedule) = config.schedule {
        let on_block = root
            .entry("on".into())
            .or_insert_with(|| Value::Mapping(Default::default()))
            .as_mapping_mut()
            .ok_or_else(|| {
                AppError::Validation(
                    "Could not preserve schedule: 'on' key in workflow kit is not a mapping."
                        .into(),
                )
            })?;

        on_block.insert("schedule".into(), schedule.clone());
    }

    if let Some(wait_minutes) = config.wait_minutes_default.as_ref() {
        let wait_minutes_config = root
            .get_mut("on")
            .and_then(|v| v.as_mapping_mut())
            .and_then(|on| on.get_mut("workflow_dispatch"))
            .and_then(|v| v.as_mapping_mut())
            .and_then(|wd| wd.get_mut("inputs"))
            .and_then(|v| v.as_mapping_mut())
            .and_then(|inputs| inputs.get_mut("wait_minutes"))
            .and_then(|v| v.as_mapping_mut())
            .ok_or_else(|| {
                AppError::Validation(
                    "Could not preserve wait_minutes: kit template is missing on.workflow_dispatch.inputs.wait_minutes."
                        .into(),
                )
            })?;

        wait_minutes_config.insert("default".into(), wait_minutes.clone());
    }

    serde_yaml::to_string(&yaml).map_err(|e| AppError::ParseError {
        what: "merged workflow content".to_string(),
        details: e.to_string(),
    })
}
