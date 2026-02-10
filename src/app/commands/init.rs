use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::adapters::assets::workflow_scaffold_assets::{
    WorkflowGenerateConfig, load_workflow_scaffold,
};
use crate::app::AppContext;
use crate::domain::workspace::manifest::{
    MANIFEST_FILENAME, hash_content, is_control_plane_entity_file,
};
use crate::domain::workspace::{JLO_DIR, VERSION_FILE};
use crate::domain::{AppError, ScaffoldManifest, WorkflowRunnerMode};
use crate::ports::{GitPort, RoleTemplateStore, WorkspaceStore};

/// Execute the unified init command.
///
/// Creates the `.jlo/` control plane, the `.jules/` runtime workspace, and
/// installs the workflow scaffold into `.github/`.
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
    persist_workflow_runner_mode(ctx.workspace(), mode)?;

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

    // Install workflow scaffold
    let root = ctx.workspace().resolve_path("");
    let generate_config = load_workflow_generate_config(&root)?;
    install_workflow_scaffold(&root, mode, &generate_config)?;

    Ok(())
}

/// Execute the workflow scaffold installation.
pub fn install_workflow_scaffold(
    root: &Path,
    mode: WorkflowRunnerMode,
    generate_config: &WorkflowGenerateConfig,
) -> Result<(), AppError> {
    let scaffold = load_workflow_scaffold(mode, generate_config)?;

    for action_dir in &scaffold.action_dirs {
        let destination = root.join(action_dir);
        if destination.exists() {
            if destination.is_dir() {
                fs::remove_dir_all(&destination)?;
            } else {
                fs::remove_file(&destination)?;
            }
        }
    }

    for file in &scaffold.files {
        let destination = root.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&destination, &file.content)?;
    }

    Ok(())
}

#[derive(Deserialize)]
struct WorkflowGenerateConfigDto {
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
    runner_mode: Option<String>,
    cron: Option<Vec<String>>,
    wait_minutes_default: Option<u32>,
}

fn parse_workflow_runner_mode(raw: Option<&str>) -> Result<WorkflowRunnerMode, AppError> {
    let value = raw.ok_or_else(|| {
        AppError::Validation("Missing workflow.runner_mode in .jlo/config.toml.".into())
    })?;
    value.parse::<WorkflowRunnerMode>().map_err(|_| {
        AppError::Validation(format!(
            "Invalid workflow.runner_mode '{}'. Expected 'remote' or 'self-hosted'.",
            value
        ))
    })
}

fn persist_workflow_runner_mode(
    workspace: &impl WorkspaceStore,
    mode: WorkflowRunnerMode,
) -> Result<(), AppError> {
    let config_path = ".jlo/config.toml";
    let content = workspace.read_file(config_path)?;
    let desired = format!("runner_mode = \"{}\"", mode.label());

    let updated = if content.contains("runner_mode = \"remote\"") {
        content.replacen("runner_mode = \"remote\"", &desired, 1)
    } else if content.contains("runner_mode = \"self-hosted\"") {
        content.replacen("runner_mode = \"self-hosted\"", &desired, 1)
    } else {
        return Err(AppError::Validation(
            "Missing workflow.runner_mode in scaffold .jlo/config.toml.".into(),
        ));
    };

    workspace.write_file(config_path, &updated)
}

fn load_workflow_config_dto(root: &Path) -> Result<WorkflowGenerateConfigDto, AppError> {
    let config_path = root.join(".jlo/config.toml");
    if !config_path.exists() {
        return Err(AppError::Validation(
            "Missing .jlo/config.toml. Run 'jlo init' to create the control plane first.".into(),
        ));
    }

    let content = fs::read_to_string(&config_path)?;
    let dto: WorkflowGenerateConfigDto = toml::from_str(&content)?;
    Ok(dto)
}

/// Read workflow generate configuration from `.jlo/config.toml` at the given repository root.
///
/// Errors on missing or invalid configuration to avoid silent fallbacks.
pub fn load_workflow_generate_config(root: &Path) -> Result<WorkflowGenerateConfig, AppError> {
    let dto = load_workflow_config_dto(root)?;

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

    let schedule_crons = raw_crons
        .into_iter()
        .map(|cron| {
            let trimmed = cron.trim();
            if trimmed.is_empty() {
                Err(AppError::Validation("workflow.cron entries must be non-empty strings.".into()))
            } else {
                Ok(trimmed.to_string())
            }
        })
        .collect::<Result<Vec<String>, _>>()?;

    let wait_minutes_default = workflow.wait_minutes_default.ok_or_else(|| {
        AppError::Validation("Missing workflow.wait_minutes_default in .jlo/config.toml.".into())
    })?;

    Ok(WorkflowGenerateConfig {
        target_branch,
        worker_branch,
        schedule_crons,
        wait_minutes_default,
    })
}

/// Read workflow runner mode from `.jlo/config.toml` at the given repository root.
///
/// The control-plane configuration is the authoritative source for selecting
/// remote vs self-hosted workflow scaffolds.
pub fn load_workflow_runner_mode(root: &Path) -> Result<WorkflowRunnerMode, AppError> {
    let dto = load_workflow_config_dto(root)?;
    let workflow = dto.workflow.ok_or_else(|| {
        AppError::Validation("Missing [workflow] section in .jlo/config.toml.".into())
    })?;
    parse_workflow_runner_mode(workflow.runner_mode.as_deref())
}
