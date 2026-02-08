use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_yaml::Value;

use crate::adapters::assets::workflow_kit_assets::{WorkflowBranchConfig, load_workflow_kit};
use crate::domain::{AppError, WorkflowRunnerMode};

/// The workflow file whose schedule should be preserved during overwrite.
const SCHEDULE_PRESERVE_FILE: &str = ".github/workflows/jules-workflows.yml";

/// Execute the workflow kit installation.
pub fn execute_workflows(
    root: &Path,
    mode: WorkflowRunnerMode,
    branches: &WorkflowBranchConfig,
) -> Result<(), AppError> {
    let kit = load_workflow_kit(mode, branches)?;

    // Parse existing workflow config before mutating any files.
    // This prevents partial deletion when the existing workflow has invalid YAML.
    let preserved_config = extract_preserved_config(root)?;

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

/// Detect the workflow runner mode from the existing workflow kit.
pub fn detect_runner_mode(root: &Path) -> Result<WorkflowRunnerMode, AppError> {
    let workflow_path = root.join(SCHEDULE_PRESERVE_FILE);
    if !workflow_path.exists() {
        return Err(AppError::Validation(
            "Workflow kit not found. Run 'jlo init' to install workflows before updating.".into(),
        ));
    }

    let content = fs::read_to_string(&workflow_path)?;
    let yaml: Value = serde_yaml::from_str(&content).map_err(|e| AppError::ParseError {
        what: format!("workflow file '{}'", SCHEDULE_PRESERVE_FILE),
        details: e.to_string(),
    })?;

    let jobs = yaml
        .get("jobs")
        .and_then(|v| v.as_mapping())
        .ok_or_else(|| AppError::Validation("Workflow kit is missing jobs section.".into()))?;

    let mut has_self_hosted = false;
    let mut has_ubuntu = false;

    let mut check_label = |label: &str| {
        if label == "self-hosted" {
            has_self_hosted = true;
        }
        if label == "ubuntu-latest" {
            has_ubuntu = true;
        }
    };

    for job in jobs.values() {
        let runs_on = job.get("runs-on");
        if let Some(Value::String(label)) = runs_on {
            check_label(label);
        } else if let Some(Value::Sequence(seq)) = runs_on {
            for item in seq {
                if let Value::String(label) = item {
                    check_label(label);
                }
            }
        }
    }

    match (has_self_hosted, has_ubuntu) {
        (true, false) => Ok(WorkflowRunnerMode::SelfHosted),
        (false, true) => Ok(WorkflowRunnerMode::Remote),
        (false, false) => {
            Err(AppError::Validation("Could not detect runner mode from workflow kit.".into()))
        }
        (true, true) => Err(AppError::Validation(
            "Workflow kit uses mixed runner labels; runner mode is ambiguous.".into(),
        )),
    }
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

// ─── Branch config from .jlo/config.toml ────────────────────────────────────

#[derive(Deserialize, Default)]
struct BranchConfigDto {
    #[serde(default)]
    run: BranchRunDto,
}

#[derive(Deserialize)]
struct BranchRunDto {
    #[serde(default = "default_branch")]
    default_branch: String,
    #[serde(default = "default_jules_branch")]
    jules_branch: String,
}

impl Default for BranchRunDto {
    fn default() -> Self {
        Self { default_branch: default_branch(), jules_branch: default_jules_branch() }
    }
}

fn default_branch() -> String {
    "main".to_string()
}
fn default_jules_branch() -> String {
    "jules".to_string()
}

/// Read branch configuration from `.jlo/config.toml` at the given repository root.
///
/// Falls back to defaults when the config file is absent or unparseable.
pub fn load_branch_config(root: &Path) -> WorkflowBranchConfig {
    let config_path = root.join(".jlo/config.toml");
    let dto: BranchConfigDto = config_path
        .exists()
        .then(|| fs::read_to_string(&config_path).ok())
        .flatten()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default();

    WorkflowBranchConfig {
        target_branch: dto.run.default_branch,
        worker_branch: dto.run.jules_branch,
    }
}
