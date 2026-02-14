use serde::Deserialize;

use crate::adapters::assets::workflow_scaffold_assets::WorkflowGenerateConfig;
use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::WorkspaceStore;

#[derive(Deserialize)]
struct WorkflowGenerateConfigDto {
    run: Option<WorkflowRunDto>,
    workflow: Option<WorkflowTimingDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct WorkflowRunDto {
    jlo_target_branch: Option<String>,
    jules_worker_branch: Option<String>,
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

fn load_workflow_config_dto(
    workspace: &impl WorkspaceStore,
) -> Result<WorkflowGenerateConfigDto, AppError> {
    let config_path = ".jlo/config.toml";
    if !workspace.file_exists(config_path) {
        return Err(AppError::Validation(
            "Missing .jlo/config.toml. Run 'jlo init' to create the control plane first.".into(),
        ));
    }

    let content = workspace.read_file(config_path)?;
    let dto: WorkflowGenerateConfigDto = toml::from_str(&content)?;
    Ok(dto)
}

/// Read workflow generate configuration from `.jlo/config.toml`.
///
/// Errors on missing or invalid configuration to avoid silent fallbacks.
pub fn load_workflow_generate_config(
    workspace: &impl WorkspaceStore,
) -> Result<WorkflowGenerateConfig, AppError> {
    let dto = load_workflow_config_dto(workspace)?;

    let run = dto
        .run
        .ok_or_else(|| AppError::Validation("Missing [run] section in .jlo/config.toml.".into()))?;
    let workflow = dto.workflow.ok_or_else(|| {
        AppError::Validation("Missing [workflow] section in .jlo/config.toml.".into())
    })?;

    let target_branch = run.jlo_target_branch.ok_or_else(|| {
        AppError::Validation("Missing run.jlo_target_branch in .jlo/config.toml.".into())
    })?;
    let worker_branch = run.jules_worker_branch.ok_or_else(|| {
        AppError::Validation("Missing run.jules_worker_branch in .jlo/config.toml.".into())
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

/// Read workflow runner mode from `.jlo/config.toml`.
///
/// The control-plane configuration is the authoritative source for selecting
/// remote vs self-hosted workflow scaffolds.
pub fn load_workflow_runner_mode(
    workspace: &impl WorkspaceStore,
) -> Result<WorkflowRunnerMode, AppError> {
    let dto = load_workflow_config_dto(workspace)?;
    let workflow = dto.workflow.ok_or_else(|| {
        AppError::Validation("Missing [workflow] section in .jlo/config.toml.".into())
    })?;
    parse_workflow_runner_mode(workflow.runner_mode.as_deref())
}

fn parse_workflow_runner_mode(raw: Option<&str>) -> Result<WorkflowRunnerMode, AppError> {
    let value = raw.ok_or_else(|| {
        AppError::Validation("Missing workflow.runner_mode in .jlo/config.toml.".into())
    })?;
    value.parse::<WorkflowRunnerMode>()
}

pub fn persist_workflow_runner_mode(
    workspace: &impl WorkspaceStore,
    mode: &WorkflowRunnerMode,
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
jlo_target_branch = "main"
jules_worker_branch = "jules"

[workflow]
runner_mode = "remote" # keep me
cron = ["0 20 * * *"]
wait_minutes_default = 30
"#;
        workspace.write_file(".jlo/config.toml", config).unwrap();

        persist_workflow_runner_mode(&workspace, &WorkflowRunnerMode::self_hosted()).unwrap();
        let updated = fs::read_to_string(temp.path().join(".jlo/config.toml")).unwrap();

        assert!(updated.contains("runner_mode = \"self-hosted\" # keep me"));
        assert!(updated.contains("jlo_target_branch = \"main\""));
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
jlo_target_branch = "main"
jules_worker_branch = "jules"
"#,
            )
            .unwrap();

        let err =
            persist_workflow_runner_mode(&workspace, &WorkflowRunnerMode::remote()).unwrap_err();
        assert!(err.to_string().contains("Missing [workflow] section"));
    }
}
