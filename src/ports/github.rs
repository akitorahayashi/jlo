use crate::domain::AppError;

pub trait GitHubPort {
    /// Dispatch a workflow via generic inputs.
    fn dispatch_workflow(
        &self,
        workflow_name: &str,
        inputs: &[(&str, &str)],
    ) -> Result<(), AppError>;
}
