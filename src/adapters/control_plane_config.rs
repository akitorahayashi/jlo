use crate::domain::config::WorkflowGenerateConfig;
use crate::domain::config::parse_config_content;
use crate::domain::{AppError, RunConfig, WorkflowRunnerMode};
use crate::ports::RepositoryFilesystem;

fn load_run_config(repository: &impl RepositoryFilesystem) -> Result<RunConfig, AppError> {
    let config_path = ".jlo/config.toml";
    if !repository.file_exists(config_path) {
        return Err(AppError::Validation(
            "Missing .jlo/config.toml. Run 'jlo init' to create the control plane first.".into(),
        ));
    }

    let content = repository.read_file(config_path)?;
    parse_config_content(&content)
}

/// Read workflow generate configuration from `.jlo/config.toml`.
///
/// Errors on missing or invalid configuration to avoid silent fallbacks.
pub fn load_workflow_generate_config(
    repository: &impl RepositoryFilesystem,
) -> Result<WorkflowGenerateConfig, AppError> {
    let config = load_run_config(repository)?;
    let workflow = config.workflow;

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
        target_branch: config.run.jlo_target_branch,
        worker_branch: config.run.jules_worker_branch,
        schedule_crons,
        wait_minutes_default,
    })
}

/// Read workflow runner mode from `.jlo/config.toml`.
///
/// The control-plane configuration is the authoritative source for selecting
/// remote vs self-hosted workflow scaffolds.
pub fn load_workflow_runner_mode(
    repository: &impl RepositoryFilesystem,
) -> Result<WorkflowRunnerMode, AppError> {
    let config = load_run_config(repository)?;
    let workflow = config.workflow;
    parse_workflow_runner_mode(workflow.runner_mode.as_deref())
}

fn parse_workflow_runner_mode(raw: Option<&str>) -> Result<WorkflowRunnerMode, AppError> {
    let value = raw.ok_or_else(|| {
        AppError::Validation("Missing workflow.runner_mode in .jlo/config.toml.".into())
    })?;
    value.parse::<WorkflowRunnerMode>()
}

pub fn persist_workflow_runner_mode(
    repository: &impl RepositoryFilesystem,
    mode: &WorkflowRunnerMode,
) -> Result<(), AppError> {
    let config_path = ".jlo/config.toml";
    let content = repository.read_file(config_path)?;
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| AppError::Validation(format!("Failed to parse .jlo/config.toml: {}", e)))?;

    let desired_value = mode.label();

    let workflow_table = doc["workflow"].as_table_mut().ok_or_else(|| {
        AppError::Validation("Missing [workflow] section in .jlo/config.toml.".into())
    })?;

    if !workflow_table.contains_key("runner_mode") {
        return Err(AppError::Validation(
            "Missing workflow.runner_mode in .jlo/config.toml.".into(),
        ));
    }

    let item = &mut workflow_table["runner_mode"];
    if let Some(current_val) = item.as_value_mut() {
        let mut new_val = toml_edit::Value::from(desired_value);
        *new_val.decor_mut() = current_val.decor().clone();
        *current_val = new_val;
    } else {
        *item = toml_edit::value(desired_value);
    }

    repository.write_file(config_path, &doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::local_repository::LocalRepositoryAdapter;
    use crate::ports::RepositoryFilesystem;
    use assert_fs::TempDir;
    use std::fs;

    #[test]
    fn persist_workflow_runner_mode_updates_only_workflow_value() {
        let temp = TempDir::new().unwrap();
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        let config = r#"# heading
[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[workflow]
runner_mode = "remote" # keep me
cron = ["0 20 * * *"]
wait_minutes_default = 30
"#;
        repository.write_file(".jlo/config.toml", config).unwrap();

        persist_workflow_runner_mode(&repository, &WorkflowRunnerMode::self_hosted()).unwrap();
        let updated = fs::read_to_string(temp.path().join(".jlo/config.toml")).unwrap();

        assert!(updated.contains("runner_mode = \"self-hosted\" # keep me"));
        assert!(updated.contains("jlo_target_branch = \"main\""));
        assert!(updated.contains("cron = [\"0 20 * * *\"]"));
    }

    #[test]
    fn persist_workflow_runner_mode_fails_without_workflow_section() {
        let temp = TempDir::new().unwrap();
        let repository = LocalRepositoryAdapter::new(temp.path().to_path_buf());
        repository
            .write_file(
                ".jlo/config.toml",
                r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"
"#,
            )
            .unwrap();

        let err =
            persist_workflow_runner_mode(&repository, &WorkflowRunnerMode::remote()).unwrap_err();
        assert!(err.to_string().contains("Missing [workflow] section"));
    }
}
