//! Workflow scaffold generate command.
//!
//! Generates the workflow scaffold with config-driven branch values. Default output
//! writes directly to the repository `.github/` directory, overwriting
//! jlo-managed files. Use `-o, --output-dir` to redirect output elsewhere.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::catalogs::workflow_scaffold::load_workflow_scaffold;
use crate::adapters::control_plane_config::load_workflow_generate_config;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::{AppError, WorkflowRunnerMode};

const SCHEMA_VERSION: u32 = 1;

/// Options for workflow generate command.
#[derive(Debug, Clone)]
pub struct WorkflowGenerateOptions {
    /// Runner mode for the workflow scaffold.
    pub mode: WorkflowRunnerMode,
    /// Output directory override. When absent, generates to repository root.
    pub output_dir: Option<PathBuf>,
}

/// Output of workflow generate command.
#[derive(Debug, Serialize)]
pub struct WorkflowGenerateOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Runner mode label.
    pub mode: String,
    /// Output directory for generated files.
    pub output_dir: String,
    /// Number of files written.
    pub file_count: usize,
}

/// Execute workflow generate command.
pub fn execute(options: WorkflowGenerateOptions) -> Result<WorkflowGenerateOutput, AppError> {
    let repo_root = find_repo_root(&std::env::current_dir()?)?;
    let repository = LocalRepositoryAdapter::new(repo_root.clone());
    let generate_config = load_workflow_generate_config(&repository)?;
    let output_dir = resolve_output_dir(&options, &repo_root)?;

    prepare_output_dir(&output_dir)?;

    let scaffold = load_workflow_scaffold(&options.mode, &generate_config)?;
    write_workflow_scaffold(&output_dir, &scaffold)?;

    Ok(WorkflowGenerateOutput {
        schema_version: SCHEMA_VERSION,
        mode: options.mode.label().to_string(),
        output_dir: output_dir.to_string_lossy().to_string(),
        file_count: scaffold.files.len(),
    })
}

fn resolve_output_dir(
    options: &WorkflowGenerateOptions,
    repo_root: &Path,
) -> Result<PathBuf, AppError> {
    if let Some(dir) = options.output_dir.as_ref() {
        return normalize_output_dir(dir.clone());
    }

    // Default: generate directly to repository root (scaffold paths already include .github/ prefix)
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

fn write_workflow_scaffold(
    output_dir: &Path,
    scaffold: &crate::adapters::catalogs::workflow_scaffold::WorkflowScaffoldAssets,
) -> Result<(), AppError> {
    for file in &scaffold.files {
        let destination = output_dir.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, &file.content)?;
    }

    Ok(())
}
