use std::str::FromStr;

use crate::domain::AppError;

/// Runner mode for workflow kits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunnerMode {
    Remote,
    SelfHosted,
}

impl WorkflowRunnerMode {
    pub fn label(self) -> &'static str {
        match self {
            WorkflowRunnerMode::Remote => "remote",
            WorkflowRunnerMode::SelfHosted => "self-hosted",
        }
    }
}

impl FromStr for WorkflowRunnerMode {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "remote" => Ok(WorkflowRunnerMode::Remote),
            "self-hosted" => Ok(WorkflowRunnerMode::SelfHosted),
            _ => Err(AppError::Validation(format!(
                "Invalid runner mode '{}'. Expected 'remote' or 'self-hosted'.",
                s
            ))),
        }
    }
}
