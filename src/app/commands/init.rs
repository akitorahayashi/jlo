use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_yaml::Value;

use crate::adapters::assets::workflow_kit_assets::{WorkflowRenderConfig, load_workflow_kit};
use crate::app::AppContext;
use crate::domain::workspace::manifest::{
    MANIFEST_FILENAME, hash_content, is_control_plane_entity_file,
};
use crate::domain::workspace::{JLO_DIR, VERSION_FILE};
use crate::domain::{AppError, ScaffoldManifest, WorkflowRunnerMode};
use crate::ports::{GitPort, RoleTemplateStore, WorkspaceStore};

const WORKFLOW_MODE_DETECTION_FILE: &str = ".github/workflows/jules-workflows.yml";

/// Execute the unified init command.
///
/// Creates the `.jlo/` control plane, the `.jules/` runtime workspace, and
/// installs the workflow kit into `.github/`.
pub fn execute<W, R, G>(
    ctx: &AppContext<W, R>,
    git: &G,
    mode: WorkflowRunnerMode,
) -> Result<(), AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    G: GitPort,
{
    if ctx.workspace().jlo_exists() {
        return Err(AppError::WorkspaceExists);
    }

    // Reject execution on 'jules' branch — init creates the control plane which
    // belongs on the user's control branch, not the runtime branch.
    let branch = git.get_current_branch()?;
    if branch == "jules" {
        return Err(AppError::Validation(
            "Init must not be run on the 'jules' branch. The 'jules' branch is the runtime branch managed by workflow bootstrap.\nRun init on your control branch (e.g. main, development).".to_string(),
        ));
    }

    // Create .jlo/ control plane (minimal intent overlay)
    let control_plane_files = ctx.templates().control_plane_files();
    for entry in &control_plane_files {
        ctx.workspace().write_file(&entry.path, &entry.content)?;
    }

    // Write version pin to .jlo/
    let jlo_version_path = format!("{}/{}", JLO_DIR, VERSION_FILE);
    ctx.workspace().write_file(&jlo_version_path, &format!("{}\n", env!("CARGO_PKG_VERSION")))?;

    // Create managed manifest for .jlo/ default entity files
    let mut map = BTreeMap::new();
    for file in &control_plane_files {
        if is_control_plane_entity_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    let managed_manifest = ScaffoldManifest::from_map(map);
    let manifest_content = managed_manifest.to_yaml()?;
    let manifest_path = format!("{}/{}", JLO_DIR, MANIFEST_FILENAME);
    ctx.workspace().write_file(&manifest_path, &manifest_content)?;

    // Install workflow kit
    let root = ctx.workspace().resolve_path("");
    let render_config = load_workflow_render_config(&root)?;
    install_workflow_kit(&root, mode, &render_config)?;

    Ok(())
}

/// Execute the workflow kit installation.
pub fn install_workflow_kit(
    root: &Path,
    mode: WorkflowRunnerMode,
    render_config: &WorkflowRenderConfig,
) -> Result<(), AppError> {
    let kit = load_workflow_kit(mode, render_config)?;

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

        fs::write(&destination, &file.content)?;
    }

    Ok(())
}

/// Detect the workflow runner mode from the existing workflow kit.
pub fn detect_workflow_runner_mode(root: &Path) -> Result<WorkflowRunnerMode, AppError> {
    let workflow_path = root.join(WORKFLOW_MODE_DETECTION_FILE);
    if !workflow_path.exists() {
        return Err(AppError::Validation(
            "Workflow kit not found. Run 'jlo init' to install workflows before updating.".into(),
        ));
    }

    let content = fs::read_to_string(&workflow_path)?;
    let yaml: Value = serde_yaml::from_str(&content).map_err(|e| AppError::ParseError {
        what: format!("workflow file '{}'", WORKFLOW_MODE_DETECTION_FILE),
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

#[derive(Deserialize)]
struct WorkflowRenderConfigDto {
    run: Option<WorkflowRunDto>,
    workflow: Option<WorkflowTimingDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct WorkflowRunDto {
    default_branch: Option<String>,
    jules_branch: Option<String>,
    parallel: Option<bool>,
    max_parallel: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowTimingDto {
    cron: Option<Vec<String>>,
    wait_minutes_default: Option<u32>,
}

/// Read workflow render configuration from `.jlo/config.toml` at the given repository root.
///
/// Errors on missing or invalid configuration to avoid silent fallbacks.
pub fn load_workflow_render_config(root: &Path) -> Result<WorkflowRenderConfig, AppError> {
    let config_path = root.join(".jlo/config.toml");
    if !config_path.exists() {
        return Err(AppError::Validation(
            "Missing .jlo/config.toml. Run 'jlo init' to create the control plane first.".into(),
        ));
    }

    let content = fs::read_to_string(&config_path)?;
    let dto: WorkflowRenderConfigDto = toml::from_str(&content)?;

    let run = dto
        .run
        .ok_or_else(|| AppError::Validation("Missing [run] section in .jlo/config.toml.".into()))?;
    let workflow = dto.workflow.ok_or_else(|| {
        AppError::Validation("Missing [workflow] section in .jlo/config.toml.".into())
    })?;

    let target_branch = run.default_branch.ok_or_else(|| {
        AppError::Validation("Missing run.default_branch in .jlo/config.toml.".into())
    })?;
    let worker_branch = run.jules_branch.ok_or_else(|| {
        AppError::Validation("Missing run.jules_branch in .jlo/config.toml.".into())
    })?;

    let raw_crons = workflow
        .cron
        .ok_or_else(|| AppError::Validation("Missing workflow.cron in .jlo/config.toml.".into()))?;
    if raw_crons.is_empty() {
        return Err(AppError::Validation(
            "workflow.cron must contain at least one cron entry.".into(),
        ));
    }

    let mut schedule_crons = Vec::with_capacity(raw_crons.len());
    for cron in raw_crons {
        let trimmed = cron.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation(
                "workflow.cron entries must be non-empty strings.".into(),
            ));
        }
        schedule_crons.push(trimmed.to_string());
    }

    let wait_minutes_default = workflow.wait_minutes_default.ok_or_else(|| {
        AppError::Validation("Missing workflow.wait_minutes_default in .jlo/config.toml.".into())
    })?;

    Ok(WorkflowRenderConfig { target_branch, worker_branch, schedule_crons, wait_minutes_default })
}
