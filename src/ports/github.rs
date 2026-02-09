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

/// Metadata for an existing pull request.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PullRequestDetail {
    /// PR number.
    pub number: u64,
    /// Head branch name.
    pub head: String,
    /// Base branch name.
    pub base: String,
    /// Whether the PR is a draft.
    pub is_draft: bool,
    /// Whether auto-merge is already enabled.
    pub auto_merge_enabled: bool,
}

/// A single comment on a PR or issue.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PrComment {
    /// Comment ID.
    pub id: u64,
    /// Comment body text.
    pub body: String,
}

/// Information about a created issue.
#[derive(Debug, Clone)]
pub struct IssueInfo {
    /// Issue number.
    pub number: u64,
    /// Issue URL.
    pub url: String,
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

    /// Close a pull request without merging.
    #[allow(dead_code)]
    fn close_pull_request(&self, pr_number: u64) -> Result<(), AppError>;

    /// Delete a remote branch.
    #[allow(dead_code)]
    fn delete_branch(&self, branch: &str) -> Result<(), AppError>;

    // === Issue operations ===

    /// Create a GitHub issue with the given title, body, and labels.
    fn create_issue(&self, title: &str, body: &str, labels: &[&str])
    -> Result<IssueInfo, AppError>;

    // === PR event operations ===

    /// Retrieve metadata for an existing pull request.
    #[allow(dead_code)]
    fn get_pr_detail(&self, pr_number: u64) -> Result<PullRequestDetail, AppError>;

    /// List comments on a pull request.
    #[allow(dead_code)]
    fn list_pr_comments(&self, pr_number: u64) -> Result<Vec<PrComment>, AppError>;

    /// Create a new comment on a pull request. Returns the comment ID.
    #[allow(dead_code)]
    fn create_pr_comment(&self, pr_number: u64, body: &str) -> Result<u64, AppError>;

    /// Update an existing PR comment by ID.
    #[allow(dead_code)]
    fn update_pr_comment(&self, comment_id: u64, body: &str) -> Result<(), AppError>;

    /// Ensure a label exists on the repository.
    /// If `color` is `Some`, sets the label color; otherwise lets GitHub assign one.
    #[allow(dead_code)]
    fn ensure_label(&self, label: &str, color: Option<&str>) -> Result<(), AppError>;

    /// Add a label to a pull request.
    #[allow(dead_code)]
    fn add_label_to_pr(&self, pr_number: u64, label: &str) -> Result<(), AppError>;

    /// Add a label to an issue.
    #[allow(dead_code)]
    fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), AppError>;

    /// Enable auto-merge (squash) on a pull request.
    #[allow(dead_code)]
    fn enable_automerge(&self, pr_number: u64) -> Result<(), AppError>;

    /// List files changed by a pull request (relative paths).
    #[allow(dead_code)]
    fn list_pr_files(&self, pr_number: u64) -> Result<Vec<String>, AppError>;
}
