//! Workflow orchestration commands for GitHub Actions integration.
//!
//! This module provides machine I/O primitives that remain usable outside GitHub Actions
//! (e.g. self-hosted workers), while keeping workflow YAML thin.

pub mod bootstrap;
mod doctor;
pub mod exchange;
pub mod generate;
mod output;
pub mod process;
pub mod push;
mod run;

pub use bootstrap::{
    WorkflowBootstrapManagedFilesOptions, WorkflowBootstrapManagedFilesOutput,
    WorkflowBootstrapWorkerBranchOptions, WorkflowBootstrapWorkerBranchOutput,
};
pub use doctor::{WorkflowDoctorOptions, WorkflowDoctorOutput};
pub use generate::{WorkflowGenerateOptions, WorkflowGenerateOutput};
pub use output::write_workflow_output;
pub use run::{WorkflowRunOptions, WorkflowRunOutput};

use crate::domain::AppError;
use crate::ports::JulesStore;

/// Execute workflow bootstrap managed files projection.
pub fn bootstrap_managed_files(
    options: WorkflowBootstrapManagedFilesOptions,
) -> Result<WorkflowBootstrapManagedFilesOutput, AppError> {
    bootstrap::managed_files::execute(options)
}

/// Execute workflow bootstrap worker-branch synchronization.
pub fn bootstrap_worker_branch(
    options: WorkflowBootstrapWorkerBranchOptions,
) -> Result<WorkflowBootstrapWorkerBranchOutput, AppError> {
    bootstrap::worker_branch::execute(options)
}

/// Execute workflow doctor validation.
pub fn doctor(options: WorkflowDoctorOptions) -> Result<WorkflowDoctorOutput, AppError> {
    doctor::execute(options)
}

/// Execute workflow run command.
pub fn run(options: WorkflowRunOptions) -> Result<WorkflowRunOutput, AppError> {
    let store = crate::adapters::local_repository::LocalRepositoryAdapter::current()?;

    let jules_path = store.jules_path();
    let git_root = jules_path.parent().unwrap_or(&jules_path).to_path_buf();
    let git = crate::adapters::git::GitCommandAdapter::new(git_root);
    let github = crate::adapters::github::GitHubCommandAdapter::new();

    run::execute(&store, options, &git, &github)
}

/// Execute workflow generate command.
pub fn generate(options: WorkflowGenerateOptions) -> Result<WorkflowGenerateOutput, AppError> {
    generate::execute(options)
}
