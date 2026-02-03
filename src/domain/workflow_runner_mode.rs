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
