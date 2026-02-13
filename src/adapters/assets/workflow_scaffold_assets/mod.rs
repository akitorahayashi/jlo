mod action_dirs;
mod asset_collect;
mod render_plan;
mod template_engine;

use include_dir::{Dir, include_dir};
use minijinja::context;

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::ScaffoldFile;

use self::action_dirs::collect_action_dirs;
use self::asset_collect::{AssetSourceFile, collect_asset_sources};
use self::render_plan::should_render_asset;
use self::template_engine::{build_template_environment, render_template_by_name};

static WORKFLOWS_ASSET_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/github");

/// Workflow generate configuration for template expansion.
///
/// Values are sourced from `.jlo/config.toml` and rendered
/// as static literals in generated workflow YAML.
#[derive(Debug, Clone)]
pub struct WorkflowGenerateConfig {
    /// Target/control branch (e.g. `main`). Maps to `run.jlo_target_branch`.
    pub target_branch: String,
    /// Worker branch hosting `.jules/` runtime state. Maps to `run.jules_worker_branch`.
    pub worker_branch: String,
    /// Cron entries used for workflow schedule.
    pub schedule_crons: Vec<String>,
    /// Default wait minutes for orchestration pacing.
    pub wait_minutes_default: u32,
}

impl Default for WorkflowGenerateConfig {
    fn default() -> Self {
        Self {
            target_branch: "main".to_string(),
            worker_branch: "jules".to_string(),
            schedule_crons: vec!["0 20 * * *".to_string()],
            wait_minutes_default: 30,
        }
    }
}

#[derive(Debug)]
pub struct WorkflowScaffoldAssets {
    pub files: Vec<ScaffoldFile>,
    pub action_dirs: Vec<String>,
}

pub fn load_workflow_scaffold(
    mode: &WorkflowRunnerMode,
    generate_config: &WorkflowGenerateConfig,
) -> Result<WorkflowScaffoldAssets, AppError> {
    let sources = collect_asset_sources(&WORKFLOWS_ASSET_DIR)?;
    if sources.is_empty() {
        return Err(AppError::InternalError(format!(
            "Workflow scaffold assets are empty for mode '{}'",
            mode.label()
        )));
    }

    let env = build_template_environment(&sources)?;

    let runner = mode.runner_label();
    let ctx = context! {
        runner => runner,
        target_branch => &generate_config.target_branch,
        worker_branch => &generate_config.worker_branch,
        workflow_schedule_crons => &generate_config.schedule_crons,
        workflow_wait_minutes_default => generate_config.wait_minutes_default,
    };

    let mut files = render_scaffold_files(&sources, &env, &ctx)?;

    for file in &mut files {
        file.path = format!(".github/{}", file.path);
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    if files.is_empty() {
        return Err(AppError::InternalError(format!(
            "No renderable workflow scaffold assets for mode '{}'",
            mode.label()
        )));
    }

    let action_dirs = collect_action_dirs(&files);
    Ok(WorkflowScaffoldAssets { files, action_dirs })
}

fn render_scaffold_files(
    sources: &[AssetSourceFile],
    env: &minijinja::Environment<'_>,
    ctx: &minijinja::Value,
) -> Result<Vec<ScaffoldFile>, AppError> {
    let mut files = Vec::new();

    for source in sources {
        if !should_render_asset(source) {
            continue;
        }

        let rendered_content = if source.is_template() {
            render_template_by_name(env, source.template_name(), ctx)?
        } else {
            source.content.clone()
        };

        files.push(ScaffoldFile { path: source.output_path(), content: rendered_content });
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_scaffold_assets_load() {
        let generate_config = WorkflowGenerateConfig::default();
        let remote = load_workflow_scaffold(&WorkflowRunnerMode::remote(), &generate_config)
            .expect("remote assets");
        assert!(!remote.files.is_empty(), "remote scaffold should have files");

        let self_hosted =
            load_workflow_scaffold(&WorkflowRunnerMode::self_hosted(), &generate_config)
                .expect("self-hosted assets");
        assert!(!self_hosted.files.is_empty(), "self-hosted scaffold should have files");
    }
}
