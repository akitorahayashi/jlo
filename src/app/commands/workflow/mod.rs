//! Workflow orchestration commands for GitHub Actions integration.
//!
//! This module provides machine I/O primitives that remain usable outside GitHub Actions
//! (e.g. self-hosted workers), while keeping workflow YAML thin.

mod doctor;
mod output;

pub use doctor::{WorkflowDoctorOptions, WorkflowDoctorOutput};
pub use output::write_workflow_output;

use crate::domain::AppError;

/// Execute workflow doctor validation.
pub fn doctor(options: WorkflowDoctorOptions) -> Result<WorkflowDoctorOutput, AppError> {
    doctor::execute(options)
}
