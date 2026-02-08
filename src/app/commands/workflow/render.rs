//! Workflow kit render command.
//!
//! Renders the workflow kit with config-driven branch values. Default output
//! writes directly to the repository `.github/` directory, overwriting
//! jlo-managed files. Use `-o, --output-dir` to redirect output elsewhere.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::assets::workflow_kit_assets::load_workflow_kit;
use crate::app::commands::init::load_workflow_render_config;
use crate::domain::{AppError, WorkflowRunnerMode};

const SCHEMA_VERSION: u32 = 1;

/// Options for workflow render command.
#[derive(Debug, Clone)]
pub struct WorkflowRenderOptions {
    /// Runner mode for the workflow kit.
    pub mode: WorkflowRunnerMode,
    /// Output directory override. When absent, renders to repository root.
    pub output_dir: Option<PathBuf>,
}

/// Output of workflow render command.
#[derive(Debug, Serialize)]
pub struct WorkflowRenderOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Runner mode label.
    pub mode: String,
    /// Output directory for rendered files.
    pub output_dir: String,
    /// Number of files written.
    pub file_count: usize,
}

/// Execute workflow render command.
pub fn execute(options: WorkflowRenderOptions) -> Result<WorkflowRenderOutput, AppError> {
    let repo_root = find_repo_root(&std::env::current_dir()?)?;
    let render_config = load_workflow_render_config(&repo_root)?;
    let output_dir = resolve_output_dir(&options, &repo_root)?;

    prepare_output_dir(&output_dir)?;

    let kit = load_workflow_kit(options.mode, &render_config)?;
    write_workflow_kit(&output_dir, &kit)?;

    Ok(WorkflowRenderOutput {
        schema_version: SCHEMA_VERSION,
        mode: options.mode.label().to_string(),
        output_dir: output_dir.to_string_lossy().to_string(),
        file_count: kit.files.len(),
    })
}

fn resolve_output_dir(
    options: &WorkflowRenderOptions,
    repo_root: &Path,
) -> Result<PathBuf, AppError> {
    if let Some(dir) = options.output_dir.as_ref() {
        return normalize_output_dir(dir.clone());
    }

    // Default: render directly to repository root (kit paths already include .github/ prefix)
    Ok(repo_root.to_path_buf())
}

fn normalize_output_dir(dir: PathBuf) -> Result<PathBuf, AppError> {
    if dir.is_absolute() {
        return Ok(dir);
    }

    let current_dir = std::env::current_dir()?;
    Ok(current_dir.join(dir))
}

fn find_repo_root(start: &Path) -> Result<PathBuf, AppError> {
    let mut current = Some(start);

    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }

    Err(AppError::RepositoryDetectionFailed)
}

/// Prepare output directory. Always overwrites jlo-managed content by default.
fn prepare_output_dir(output_dir: &Path) -> Result<(), AppError> {
    if output_dir.exists() && output_dir.is_file() {
        return Err(AppError::Validation(format!(
            "Output path '{}' is a file. Provide a directory path.",
            output_dir.display()
        )));
    }

    fs::create_dir_all(output_dir)?;
    Ok(())
}

fn write_workflow_kit(
    output_dir: &Path,
    kit: &crate::adapters::assets::workflow_kit_assets::WorkflowKitAssets,
) -> Result<(), AppError> {
    for file in &kit.files {
        let destination = output_dir.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, &file.content)?;
    }

    Ok(())
}
