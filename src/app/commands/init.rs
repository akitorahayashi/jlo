use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::adapters::assets::workflow_scaffold_assets::{
    WorkflowGenerateConfig, load_workflow_scaffold,
};
use crate::app::AppContext;
use crate::domain::workspace::manifest::{hash_content, is_control_plane_entity_file};
use crate::domain::workspace::paths::jlo;
use crate::domain::workspace::{JLO_DIR, VERSION_FILE};
use crate::domain::{AppError, Layer, ScaffoldManifest, Schedule, WorkflowRunnerMode};
use crate::ports::ScaffoldFile;
use crate::ports::{GitPort, RoleTemplateStore, WorkspaceStore};
use std::collections::HashMap;

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

    let seeded_roles = seed_scheduled_roles(ctx)?;

    // Write version pin to .jlo/
    let jlo_version_path = format!("{}/{}", JLO_DIR, VERSION_FILE);
    ctx.workspace().write_file(&jlo_version_path, &format!("{}\n", env!("CARGO_PKG_VERSION")))?;

    // Create managed manifest for .jlo/ default entity files
    let mut map = BTreeMap::new();
    for file in control_plane_files.iter().chain(seeded_roles.iter()) {
        if is_control_plane_entity_file(&file.path) {
            map.insert(file.path.clone(), hash_content(&file.content));
        }
    }
    let managed_manifest = ScaffoldManifest::from_map(map);
    let manifest_content = managed_manifest.to_yaml()?;
    let manifest_path = jlo::manifest_relative();
    ctx.workspace().write_file(&manifest_path, &manifest_content)?;

    // Install workflow scaffold
    let root = ctx.workspace().resolve_path("");
    let generate_config = load_workflow_generate_config(&root)?;
    install_workflow_scaffold(&root, mode, &generate_config)?;

    Ok(())
}

fn seed_scheduled_roles<W, R>(ctx: &AppContext<W, R>) -> Result<Vec<ScaffoldFile>, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    let schedule_content = ctx.workspace().read_file(".jlo/scheduled.toml")?;
    let schedule = Schedule::parse_toml(&schedule_content)?;

    let catalog = ctx.templates().builtin_role_catalog()?;
    let mut catalog_index: HashMap<(String, String), ScaffoldFile> = HashMap::new();

    for entry in catalog {
        let content = ctx.templates().builtin_role_content(&entry.path)?;
        let path =
            format!(".jlo/roles/{}/{}/role.yml", entry.layer.dir_name(), entry.name.as_str());
        catalog_index.insert(
            (entry.layer.dir_name().to_string(), entry.name.as_str().to_string()),
            ScaffoldFile { path, content },
        );
    }

    let mut seeded = Vec::new();

    for role in &schedule.observers.roles {
        let key = (Layer::Observers.dir_name().to_string(), role.name.as_str().to_string());
        let file = catalog_index.get(&key).ok_or_else(|| {
            AppError::Validation(format!(
                "Scheduled observer role '{}' is missing from builtin catalog",
                role.name.as_str()
            ))
        })?;
        ctx.workspace().write_file(&file.path, &file.content)?;
        seeded.push(file.clone());
    }

    if let Some(ref innovators) = schedule.innovators {
        for role in &innovators.roles {
            let key = (Layer::Innovators.dir_name().to_string(), role.name.as_str().to_string());
            let file = catalog_index.get(&key).ok_or_else(|| {
                AppError::Validation(format!(
                    "Scheduled innovator role '{}' is missing from builtin catalog",
                    role.name.as_str()
                ))
            })?;
            ctx.workspace().write_file(&file.path, &file.content)?;
            seeded.push(file.clone());
        }
    }

    Ok(seeded)
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
    value.parse::<WorkflowRunnerMode>()
}

fn persist_workflow_runner_mode(
    workspace: &impl WorkspaceStore,
    mode: WorkflowRunnerMode,
) -> Result<(), AppError> {
    let config_path = ".jlo/config.toml";
    let content = workspace.read_file(config_path)?;
    let desired_value = mode.label();

    let mut updated = String::with_capacity(content.len());
    let mut in_workflow_table = false;
    let mut saw_workflow_table = false;
    let mut updated_runner_mode = false;

    for line in content.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches('\n');
        let trimmed = line_without_newline.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_workflow_table = trimmed == "[workflow]";
            if in_workflow_table {
                saw_workflow_table = true;
            }
            updated.push_str(line);
            continue;
        }

        if in_workflow_table
            && !updated_runner_mode
            && let Some(rewritten) = rewrite_runner_mode_line(line, desired_value)
        {
            updated.push_str(&rewritten);
            updated_runner_mode = true;
            continue;
        }

        updated.push_str(line);
    }

    if !saw_workflow_table {
        return Err(AppError::Validation(
            "Missing [workflow] section in scaffold .jlo/config.toml.".into(),
        ));
    }
    if !updated_runner_mode {
        return Err(AppError::Validation(
            "Missing workflow.runner_mode in scaffold .jlo/config.toml.".into(),
        ));
    }

    workspace.write_file(config_path, &updated)
}

fn rewrite_runner_mode_line(line: &str, desired_value: &str) -> Option<String> {
    let (body, newline) =
        line.strip_suffix('\n').map_or((line, ""), |line_without_nl| (line_without_nl, "\n"));

    let trimmed_start = body.trim_start();
    if !trimmed_start.starts_with("runner_mode") {
        return None;
    }

    let remainder = &trimmed_start["runner_mode".len()..];
    if !remainder.trim_start().starts_with('=') {
        return None;
    }

    let indent_len = body.len() - trimmed_start.len();
    let indent = &body[..indent_len];

    let comment_suffix = body
        .find('#')
        .map(|idx| &body[idx..])
        .filter(|comment| !comment.trim().is_empty())
        .unwrap_or("");

    let mut rewritten = format!("{indent}runner_mode = \"{desired_value}\"");
    if !comment_suffix.is_empty() {
        rewritten.push(' ');
        rewritten.push_str(comment_suffix.trim_start());
    }
    rewritten.push_str(newline);
    Some(rewritten)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
    use assert_fs::TempDir;
    use std::fs;

    #[test]
    fn persist_workflow_runner_mode_updates_only_workflow_value() {
        let temp = TempDir::new().unwrap();
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        let config = r#"# heading
[run]
default_branch = "main"
jules_branch = "jules"

[workflow]
runner_mode = "remote" # keep me
cron = ["0 20 * * *"]
wait_minutes_default = 30
"#;
        workspace.write_file(".jlo/config.toml", config).unwrap();

        persist_workflow_runner_mode(&workspace, WorkflowRunnerMode::self_hosted()).unwrap();
        let updated = fs::read_to_string(temp.path().join(".jlo/config.toml")).unwrap();

        assert!(updated.contains("runner_mode = \"self-hosted\" # keep me"));
        assert!(updated.contains("default_branch = \"main\""));
        assert!(updated.contains("cron = [\"0 20 * * *\"]"));
    }

    #[test]
    fn persist_workflow_runner_mode_fails_without_workflow_section() {
        let temp = TempDir::new().unwrap();
        let workspace = FilesystemWorkspaceStore::new(temp.path().to_path_buf());
        workspace
            .write_file(
                ".jlo/config.toml",
                r#"[run]
default_branch = "main"
jules_branch = "jules"
"#,
            )
            .unwrap();

        let err = persist_workflow_runner_mode(&workspace, WorkflowRunnerMode::remote()).unwrap_err();
        assert!(err.to_string().contains("Missing [workflow] section"));
    }
}
