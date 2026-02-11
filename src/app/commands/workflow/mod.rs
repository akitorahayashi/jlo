//! Workflow orchestration commands for GitHub Actions integration.
//!
//! This module provides machine I/O primitives that remain usable outside GitHub Actions
//! (e.g. self-hosted workers), while keeping workflow YAML thin.

pub mod bootstrap;
mod doctor;
pub mod generate;
pub mod issue;
pub mod matrix;
mod output;
pub mod pr;
mod run;
pub mod workspace;

pub use bootstrap::{WorkflowBootstrapOptions, WorkflowBootstrapOutput};
pub use doctor::{WorkflowDoctorOptions, WorkflowDoctorOutput};
pub use generate::{WorkflowGenerateOptions, WorkflowGenerateOutput};
pub use output::write_workflow_output;
pub use run::{WorkflowRunOptions, WorkflowRunOutput};

use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Execute workflow bootstrap.
pub fn bootstrap(options: WorkflowBootstrapOptions) -> Result<WorkflowBootstrapOutput, AppError> {
    bootstrap::execute(options)
}

/// Execute workflow doctor validation.
pub fn doctor(options: WorkflowDoctorOptions) -> Result<WorkflowDoctorOutput, AppError> {
    doctor::execute(options)
}

/// Execute workflow run command.
pub fn run(options: WorkflowRunOptions) -> Result<WorkflowRunOutput, AppError> {
    let store = crate::adapters::workspace_filesystem::FilesystemWorkspaceStore::current()?;

    let jules_path = store.jules_path();
    let git_root = jules_path.parent().unwrap_or(&jules_path).to_path_buf();
    let git = crate::adapters::git_command::GitCommandAdapter::new(git_root);
    let github = crate::adapters::github_command::GitHubCommandAdapter::new();

    run::execute(&store, options, &git, &github)
}

/// Execute workflow generate command.
pub fn generate(options: WorkflowGenerateOptions) -> Result<WorkflowGenerateOutput, AppError> {
    generate::execute(options)
}
