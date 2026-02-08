//! Workflow kit render command.
//!
//! Renders the workflow kit to a deterministic output directory without mutating
//! existing workflow files in the repository.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::assets::workflow_kit_assets::load_workflow_kit;
use crate::domain::{AppError, WorkflowRunnerMode};

const SCHEMA_VERSION: u32 = 1;

/// Options for workflow render command.
#[derive(Debug, Clone)]
pub struct WorkflowRenderOptions {
    /// Runner mode for the workflow kit.
    pub mode: WorkflowRunnerMode,
    /// Output directory for rendered workflow kit.
    pub output_dir: Option<PathBuf>,
    /// Overwrite output directory if it exists and is not empty.
    pub overwrite: bool,
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
    let output_dir = resolve_output_dir(&options)?;

    prepare_output_dir(&output_dir, options.overwrite)?;

    let kit = load_workflow_kit(options.mode)?;
    write_workflow_kit(&output_dir, &kit)?;

    Ok(WorkflowRenderOutput {
        schema_version: SCHEMA_VERSION,
        mode: options.mode.label().to_string(),
        output_dir: output_dir.to_string_lossy().to_string(),
        file_count: kit.files.len(),
    })
}

fn resolve_output_dir(options: &WorkflowRenderOptions) -> Result<PathBuf, AppError> {
    if let Some(dir) = options.output_dir.as_ref() {
        return normalize_output_dir(dir.clone());
    }

    let current_dir = std::env::current_dir()?;
    let repo_root = find_repo_root(&current_dir)?;

    Ok(repo_root.join(".tmp").join("workflow-kit-render").join(options.mode.label()))
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

fn prepare_output_dir(output_dir: &Path, overwrite: bool) -> Result<(), AppError> {
    if output_dir.exists() {
        if output_dir.is_file() {
            return Err(AppError::Validation(format!(
                "Output path '{}' is a file. Provide a directory path.",
                output_dir.display()
            )));
        }

        let mut entries = fs::read_dir(output_dir)?;
        let has_entries = entries.next().is_some();

        if has_entries {
            if !overwrite {
                return Err(AppError::Validation(format!(
                    "Output directory '{}' is not empty. Use --overwrite to replace it.",
                    output_dir.display()
                )));
            }
            fs::remove_dir_all(output_dir)?;
        }
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
