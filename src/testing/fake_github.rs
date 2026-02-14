use crate::domain::AppError;
use crate::ports::GitHubPort;

pub struct FakeGitHub;

impl GitHubPort for FakeGitHub {
    fn create_pull_request(
        &self,
        head: &str,
        base: &str,
        _title: &str,
        _body: &str,
    ) -> Result<crate::ports::PullRequestInfo, AppError> {
        Ok(crate::ports::PullRequestInfo {
            number: 42,
            url: "https://example.com/pr/42".into(),
            head: head.to_string(),
            base: base.to_string(),
        })
    }
    fn close_pull_request(&self, _pr_number: u64) -> Result<(), AppError> {
        Ok(())
    }
    fn delete_branch(&self, _branch: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn create_issue(
        &self,
        _title: &str,
        _body: &str,
        _labels: &[&str],
    ) -> Result<crate::ports::IssueInfo, AppError> {
        Ok(crate::ports::IssueInfo { number: 1, url: "https://example.com/issues/1".into() })
    }
    fn get_pr_detail(
        &self,
        _pr_number: u64,
    ) -> Result<crate::ports::PullRequestDetail, AppError> {
        Ok(crate::ports::PullRequestDetail {
            number: 42,
            head: String::new(),
            base: String::new(),
            is_draft: false,
            auto_merge_enabled: false,
        })
    }
    fn list_pr_comments(
        &self,
        _pr_number: u64,
    ) -> Result<Vec<crate::ports::PrComment>, AppError> {
        Ok(Vec::new())
    }
    fn create_pr_comment(&self, _pr_number: u64, _body: &str) -> Result<u64, AppError> {
        Ok(1)
    }
    fn update_pr_comment(&self, _comment_id: u64, _body: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn ensure_label(&self, _label: &str, _color: Option<&str>) -> Result<(), AppError> {
        Ok(())
    }
    fn add_label_to_pr(&self, _pr_number: u64, _label: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn add_label_to_issue(&self, _issue_number: u64, _label: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn enable_automerge(&self, _pr_number: u64) -> Result<(), AppError> {
        Ok(())
    }
    fn list_pr_files(&self, _pr_number: u64) -> Result<Vec<String>, AppError> {
        Ok(Vec::new())
    }
}
