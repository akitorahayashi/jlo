//! Workflow kit render command.
//!
//! Renders the workflow kit with config-driven branch values. Default output
//! writes directly to the repository `.github/` directory, overwriting
//! jlo-managed files. Use `-o, --output-dir` to redirect output elsewhere.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::assets::workflow_kit_assets::load_workflow_kit;
use crate::app::commands::init::load_workflow_render_config;
use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::WorkspaceStore;

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
pub fn execute(
    store: &impl WorkspaceStore,
    options: WorkflowRenderOptions,
) -> Result<WorkflowRenderOutput, AppError> {
    // We use the store's root as the base for resolution
    let repo_root = store.resolve_path("");

    // Load config (still using fs internally via helper, but scoped to repo root)
    let render_config = load_workflow_render_config(&repo_root)?;

    // Resolve output directory
    let output_dir = if let Some(dir) = options.output_dir.as_ref() {
        if dir.is_absolute() {
            dir.clone()
        } else {
            store.resolve_path(dir.to_str().unwrap_or_default())
        }
    } else {
        repo_root.clone()
    };

    prepare_output_dir(store, &output_dir)?;

    let kit = load_workflow_kit(options.mode, &render_config)?;
    write_workflow_kit(store, &output_dir, &kit)?;

    Ok(WorkflowRenderOutput {
        schema_version: SCHEMA_VERSION,
        mode: options.mode.label().to_string(),
        output_dir: output_dir.to_string_lossy().to_string(),
        file_count: kit.files.len(),
    })
}

/// Prepare output directory. Always overwrites jlo-managed content by default.
fn prepare_output_dir(store: &impl WorkspaceStore, output_dir: &Path) -> Result<(), AppError> {
    let output_dir_str = output_dir.to_str().unwrap_or_default();

    // Check if it exists and is a file
    if store.file_exists(output_dir_str) && !store.is_dir(output_dir_str) {
        return Err(AppError::Validation(format!(
            "Output path '{}' is a file. Provide a directory path.",
            output_dir.display()
        )));
    }

    store.create_dir_all(output_dir_str)?;
    Ok(())
}

fn write_workflow_kit(
    store: &impl WorkspaceStore,
    output_dir: &Path,
    kit: &crate::adapters::assets::workflow_kit_assets::WorkflowKitAssets,
) -> Result<(), AppError> {
    for file in &kit.files {
        let destination = output_dir.join(&file.path);
        // store.write_file handles parent directory creation
        store.write_file(destination.to_str().unwrap_or_default(), &file.content)?;
    }

    Ok(())
}
