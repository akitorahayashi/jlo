use std::time::Duration;

use crate::domain::AppError;

/// Information about a created pull request.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PullRequestInfo {
    /// PR number.
    pub number: u64,
    /// PR URL.
    pub url: String,
    /// Head branch name.
    pub head: String,
    /// Base branch name.
    pub base: String,
}

pub trait GitHubPort {
    /// Dispatch a workflow via generic inputs.
    fn dispatch_workflow(
        &self,
        workflow_name: &str,
        inputs: &[(&str, &str)],
    ) -> Result<(), AppError>;

    // === Mock mode operations ===

    /// Create a pull request.
    fn create_pull_request(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<PullRequestInfo, AppError>;

    /// Wait for a PR to be merged.
    fn wait_for_merge(&self, pr_number: u64, timeout: Duration) -> Result<(), AppError>;

    /// Close a pull request without merging.
    #[allow(dead_code)]
    fn close_pull_request(&self, pr_number: u64) -> Result<(), AppError>;

    /// Delete a remote branch.
    #[allow(dead_code)]
    fn delete_branch(&self, branch: &str) -> Result<(), AppError>;

    /// Enable auto-merge on a PR.
    fn enable_auto_merge(&self, pr_number: u64) -> Result<(), AppError>;
}
