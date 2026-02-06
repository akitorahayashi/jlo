//! Workflow orchestration commands for GitHub Actions integration.
//!
//! This module provides machine I/O primitives that remain usable outside GitHub Actions
//! (e.g. self-hosted workers), while keeping workflow YAML thin.

pub mod cleanup;
mod doctor;
pub mod matrix;
mod output;
mod pr_label;
mod run;

pub use cleanup::{
    WorkflowCleanupMockOptions, WorkflowCleanupMockOutput, WorkflowCleanupProcessedIssueOptions,
    WorkflowCleanupProcessedIssueOutput,
};
pub use doctor::{WorkflowDoctorOptions, WorkflowDoctorOutput};
pub use output::write_workflow_output;
pub use pr_label::{WorkflowPrLabelOptions, WorkflowPrLabelOutput};
pub use run::{WorkflowRunOptions, WorkflowRunOutput};

use crate::domain::AppError;

/// Execute workflow doctor validation.
pub fn doctor(options: WorkflowDoctorOptions) -> Result<WorkflowDoctorOutput, AppError> {
    doctor::execute(options)
}

/// Execute workflow run command.
pub fn run(options: WorkflowRunOptions) -> Result<WorkflowRunOutput, AppError> {
    run::execute(options)
}

/// Execute workflow cleanup mock command.
pub fn cleanup_mock(
    options: WorkflowCleanupMockOptions,
) -> Result<WorkflowCleanupMockOutput, AppError> {
    cleanup::cleanup_mock(options)
}

/// Execute workflow cleanup processed-issue command.
pub fn cleanup_processed_issue(
    options: WorkflowCleanupProcessedIssueOptions,
) -> Result<WorkflowCleanupProcessedIssueOutput, AppError> {
    cleanup::cleanup_processed_issue(options)
}

/// Execute workflow pr label-from-branch command.
pub fn pr_label_from_branch(
    options: WorkflowPrLabelOptions,
) -> Result<WorkflowPrLabelOutput, AppError> {
    pr_label::execute(options)
}
