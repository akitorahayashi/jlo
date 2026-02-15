use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::domain::AppError;
use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};

pub struct FakeGitHub {
    pub pr_detail: Mutex<PullRequestDetail>,
    pub comments: Mutex<Vec<PrComment>>,
    pub created_issues: Mutex<Vec<(String, String)>>,
    pub ensured_labels: Mutex<Vec<String>>,
    pub applied_labels: Mutex<Vec<(u64, String)>>,
    pub files: Mutex<Vec<String>>,

    // Auto-merge simulation
    pub automerge_calls: AtomicU32,
    pub remaining_transient_automerge_failures: AtomicU32,
    pub fatal_automerge_failure: AtomicBool,
    pub set_automerge_enabled_on_first_error: AtomicBool,

    // ID generation
    pub next_comment_id: AtomicU64,
    pub next_issue_number: AtomicU64,
    pub next_pr_number: AtomicU64,
}

impl Default for FakeGitHub {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeGitHub {
    pub fn new() -> Self {
        Self {
            pr_detail: Mutex::new(PullRequestDetail {
                number: 1,
                head: "feature/test".to_string(),
                base: "main".to_string(),
                is_draft: false,
                auto_merge_enabled: false,
            }),
            comments: Mutex::new(Vec::new()),
            created_issues: Mutex::new(Vec::new()),
            ensured_labels: Mutex::new(Vec::new()),
            applied_labels: Mutex::new(Vec::new()),
            files: Mutex::new(Vec::new()),
            automerge_calls: AtomicU32::new(0),
            remaining_transient_automerge_failures: AtomicU32::new(0),
            fatal_automerge_failure: AtomicBool::new(false),
            set_automerge_enabled_on_first_error: AtomicBool::new(false),
            next_comment_id: AtomicU64::new(100),
            next_issue_number: AtomicU64::new(1),
            next_pr_number: AtomicU64::new(101),
        }
    }

    pub fn with_pr_detail(self, detail: PullRequestDetail) -> Self {
        *self.pr_detail.lock().unwrap() = detail;
        self
    }

    pub fn with_files(self, files: Vec<String>) -> Self {
        *self.files.lock().unwrap() = files;
        self
    }

    // Helper from process.rs tests
    pub fn jules_runtime_pr() -> Self {
        Self::new()
            .with_pr_detail(PullRequestDetail {
                number: 42,
                head: "jules-observer-abc123".to_string(),
                base: "jules".to_string(),
                is_draft: false,
                auto_merge_enabled: false,
            })
            .with_files(vec![".jules/exchange/events/pending/state.yml".to_string()])
    }

    pub fn non_jules_pr() -> Self {
        Self::new()
            .with_pr_detail(PullRequestDetail {
                number: 99,
                head: "feature/foo".to_string(),
                base: "main".to_string(),
                is_draft: false,
                auto_merge_enabled: false,
            })
            .with_files(vec!["src/main.rs".to_string()])
    }

    pub fn with_transient_automerge_failures(self, count: u32) -> Self {
        self.remaining_transient_automerge_failures.store(count, Ordering::SeqCst);
        self
    }

    pub fn with_fatal_automerge_failure(self) -> Self {
        self.fatal_automerge_failure.store(true, Ordering::SeqCst);
        self
    }

    pub fn with_race_automerge_state_after_first_failure(self) -> Self {
        self.remaining_transient_automerge_failures.store(1, Ordering::SeqCst);
        self.set_automerge_enabled_on_first_error.store(true, Ordering::SeqCst);
        self
    }
}

impl GitHubPort for FakeGitHub {
    fn create_pull_request(
        &self,
        head: &str,
        base: &str,
        _title: &str,
        _body: &str,
    ) -> Result<PullRequestInfo, AppError> {
        let number = self.next_pr_number.fetch_add(1, Ordering::SeqCst);
        Ok(PullRequestInfo {
            number,
            url: format!("https://example.com/pr/{}", number),
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
        title: &str,
        body: &str,
        _labels: &[&str],
    ) -> Result<IssueInfo, AppError> {
        let number = self.next_issue_number.fetch_add(1, Ordering::SeqCst);
        self.created_issues.lock().unwrap().push((title.to_string(), body.to_string()));
        Ok(IssueInfo { number, url: format!("https://example.com/issues/{}", number) })
    }

    fn get_pr_detail(&self, _pr_number: u64) -> Result<PullRequestDetail, AppError> {
        Ok(self.pr_detail.lock().unwrap().clone())
    }

    fn list_pr_comments(&self, _pr_number: u64) -> Result<Vec<PrComment>, AppError> {
        Ok(self.comments.lock().unwrap().clone())
    }

    fn create_pr_comment(&self, _pr_number: u64, body: &str) -> Result<u64, AppError> {
        let id = self.next_comment_id.fetch_add(1, Ordering::SeqCst);
        self.comments.lock().unwrap().push(PrComment { id, body: body.to_string() });
        Ok(id)
    }

    fn update_pr_comment(&self, id: u64, body: &str) -> Result<(), AppError> {
        let mut comments = self.comments.lock().unwrap();
        if let Some(c) = comments.iter_mut().find(|c| c.id == id) {
            c.body = body.to_string();
        }
        Ok(())
    }

    fn ensure_label(&self, label: &str, _color: Option<&str>) -> Result<(), AppError> {
        self.ensured_labels.lock().unwrap().push(label.to_string());
        Ok(())
    }

    fn add_label_to_pr(&self, _pr_number: u64, _label: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), AppError> {
        self.applied_labels.lock().unwrap().push((issue_number, label.to_string()));
        Ok(())
    }

    fn enable_automerge(&self, _pr_number: u64) -> Result<(), AppError> {
        self.automerge_calls.fetch_add(1, Ordering::SeqCst);

        if self.fatal_automerge_failure.load(Ordering::SeqCst) {
            return Err(AppError::ExternalToolError {
                tool: "gh".to_string(),
                error: "gh command failed: GraphQL: Validation failed: Pull request is not in a mergeable state".to_string(),
            });
        }

        // Check transient failures
        // We use fetch_sub to decrement if > 0.
        // Or simplified logic: check load, if > 0, dec and return error.
        if self
            .remaining_transient_automerge_failures
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |val| {
                if val > 0 { Some(val - 1) } else { None }
            })
            .is_ok()
        {
            // Decremented successfully, so we simulate a failure.
            if self.set_automerge_enabled_on_first_error.load(Ordering::SeqCst) {
                // Simulate race condition where it got enabled despite error
                self.pr_detail.lock().unwrap().auto_merge_enabled = true;
                self.set_automerge_enabled_on_first_error.store(false, Ordering::SeqCst);
            }
            return Err(AppError::ExternalToolError {
                tool: "gh".to_string(),
                error: "gh command failed: GraphQL: Base branch was modified. Review and try the merge again. (mergePullRequest)".to_string(),
            });
        }

        self.pr_detail.lock().unwrap().auto_merge_enabled = true;
        Ok(())
    }

    fn list_pr_files(&self, _pr_number: u64) -> Result<Vec<String>, AppError> {
        Ok(self.files.lock().unwrap().clone())
    }
}
