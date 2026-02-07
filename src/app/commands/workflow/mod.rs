//! Workflow orchestration commands for GitHub Actions integration.
//!
//! This module provides machine I/O primitives that remain usable outside GitHub Actions
//! (e.g. self-hosted workers), while keeping workflow YAML thin.

pub mod bootstrap;
pub mod cleanup;
mod doctor;
pub mod matrix;
mod output;
mod pr_label;
mod run;
#[path = "workstreams/mod.rs"]
pub mod workstreams;

pub use bootstrap::{WorkflowBootstrapOptions, WorkflowBootstrapOutput};
pub use cleanup::{WorkflowCleanupMockOptions, WorkflowCleanupMockOutput};
pub use doctor::{WorkflowDoctorOptions, WorkflowDoctorOutput};
pub use output::write_workflow_output;
pub use pr_label::{WorkflowPrLabelOptions, WorkflowPrLabelOutput};
pub use run::{WorkflowRunOptions, WorkflowRunOutput};
pub use workstreams::{
    WorkflowWorkstreamsCleanIssueOptions, WorkflowWorkstreamsCleanIssueOutput,
    WorkflowWorkstreamsInspectOptions, WorkflowWorkstreamsInspectOutput,
    WorkflowWorkstreamsPublishProposalsOptions, WorkflowWorkstreamsPublishProposalsOutput,
};

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

/// Execute workflow cleanup mock command.
pub fn cleanup_mock(
    options: WorkflowCleanupMockOptions,
) -> Result<WorkflowCleanupMockOutput, AppError> {
    cleanup::cleanup_mock(options)
}

/// Execute workflow workstreams inspect command.
pub fn workstreams_inspect(
    options: WorkflowWorkstreamsInspectOptions,
) -> Result<WorkflowWorkstreamsInspectOutput, AppError> {
    workstreams::inspect(options)
}

/// Execute workflow workstreams clean issue command.
pub fn workstreams_clean_issue(
    options: WorkflowWorkstreamsCleanIssueOptions,
) -> Result<WorkflowWorkstreamsCleanIssueOutput, AppError> {
    workstreams::clean_issue(options)
}

/// Execute workflow workstreams publish-proposals command.
pub fn workstreams_publish_proposals(
    options: WorkflowWorkstreamsPublishProposalsOptions,
) -> Result<WorkflowWorkstreamsPublishProposalsOutput, AppError> {
    workstreams::publish_proposals(options)
}

/// Execute workflow pr label-from-branch command.
pub fn pr_label_from_branch(
    options: WorkflowPrLabelOptions,
) -> Result<WorkflowPrLabelOutput, AppError> {
    pr_label::execute(options)
}
