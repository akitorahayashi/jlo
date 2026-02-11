use std::path::Path;

use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock narrator.
pub fn execute_mock_narrator<G, H, W>(
    _jules_path: &Path,
    config: &MockConfig,
    _git: &G,
    _github: &H,
    _workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore,
{
    let _ = config.branch_prefix(Layer::Narrator)?;
    println!("Mock narrator: no-op (preserving existing .jules/exchange/changes.yml)");

    Ok(MockOutput {
        mock_branch: String::new(),
        mock_pr_number: 0,
        mock_pr_url: String::new(),
        mock_tag: config.mock_tag.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{DiscoveredRole, PullRequestInfo, ScaffoldFile};
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct MustNotTouchGit;

    impl GitPort for MustNotTouchGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_head_sha");
        }

        fn get_current_branch(&self) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_current_branch");
        }

        fn commit_exists(&self, _sha: &str) -> bool {
            panic!("mock narrator no-op must not call commit_exists");
        }

        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call get_nth_ancestor");
        }

        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            panic!("mock narrator no-op must not call has_changes");
        }

        fn run_command(&self, _args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call run_command");
        }

        fn checkout_branch(&self, _branch: &str, _create: bool) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call checkout_branch");
        }

        fn push_branch(&self, _branch: &str, _force: bool) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call push_branch");
        }

        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call commit_files");
        }

        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call fetch");
        }

        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            panic!("mock narrator no-op must not call delete_branch");
        }
    }

    struct MustNotTouchGitHub;

    impl GitHubPort for MustNotTouchGitHub {
        fn dispatch_workflow(
            &self,
            _workflow_name: &str,
            _inputs: &[(&str, &str)],
        ) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call dispatch_workflow");
        }

        fn create_pull_request(
            &self,
            _head: &str,
            _base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<PullRequestInfo, AppError> {
            panic!("mock narrator no-op must not call create_pull_request");
        }

        fn close_pull_request(&self, _pr_number: u64) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call close_pull_request");
        }

        fn delete_branch(&self, _branch: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call delete_branch");
        }

        fn create_issue(
            &self,
            _title: &str,
            _body: &str,
            _labels: &[&str],
        ) -> Result<crate::ports::IssueInfo, AppError> {
            panic!("mock narrator no-op must not call create_issue");
        }

        fn get_pr_detail(
            &self,
            _pr_number: u64,
        ) -> Result<crate::ports::PullRequestDetail, AppError> {
            panic!("mock narrator no-op must not call get_pr_detail");
        }
        fn list_pr_comments(
            &self,
            _pr_number: u64,
        ) -> Result<Vec<crate::ports::PrComment>, AppError> {
            panic!("mock narrator no-op must not call list_pr_comments");
        }
        fn create_pr_comment(&self, _pr_number: u64, _body: &str) -> Result<u64, AppError> {
            panic!("mock narrator no-op must not call create_pr_comment");
        }
        fn update_pr_comment(&self, _comment_id: u64, _body: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call update_pr_comment");
        }
        fn ensure_label(&self, _label: &str, _color: Option<&str>) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call ensure_label");
        }
        fn add_label_to_pr(&self, _pr_number: u64, _label: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call add_label_to_pr");
        }
        fn add_label_to_issue(&self, _issue_number: u64, _label: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call add_label_to_issue");
        }
        fn enable_automerge(&self, _pr_number: u64) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call enable_automerge");
        }
        fn list_pr_files(&self, _pr_number: u64) -> Result<Vec<String>, AppError> {
            panic!("mock narrator no-op must not call list_pr_files");
        }
    }

    struct DummyWorkspace;

    impl crate::domain::PromptAssetLoader for DummyWorkspace {
        fn read_asset(&self, _path: &Path) -> std::io::Result<String> {
            panic!("mock narrator no-op must not call read_asset");
        }

        fn asset_exists(&self, _path: &Path) -> bool {
            panic!("mock narrator no-op must not call asset_exists");
        }

        fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
            panic!("mock narrator no-op must not call ensure_asset_dir");
        }

        fn copy_asset(&self, _from: &Path, _to: &Path) -> std::io::Result<u64> {
            panic!("mock narrator no-op must not call copy_asset");
        }
    }

    impl WorkspaceStore for DummyWorkspace {
        fn exists(&self) -> bool {
            panic!("mock narrator no-op must not call exists");
        }

        fn jlo_exists(&self) -> bool {
            panic!("mock narrator no-op must not call jlo_exists");
        }

        fn jules_path(&self) -> PathBuf {
            panic!("mock narrator no-op must not call jules_path");
        }

        fn jlo_path(&self) -> PathBuf {
            panic!("mock narrator no-op must not call jlo_path");
        }

        fn create_structure(&self, _scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call create_structure");
        }

        fn write_version(&self, _version: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call write_version");
        }

        fn read_version(&self) -> Result<Option<String>, AppError> {
            panic!("mock narrator no-op must not call read_version");
        }

        fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
            panic!("mock narrator no-op must not call discover_roles");
        }

        fn find_role_fuzzy(&self, _query: &str) -> Result<Option<DiscoveredRole>, AppError> {
            panic!("mock narrator no-op must not call find_role_fuzzy");
        }

        fn role_path(&self, _role: &DiscoveredRole) -> Option<PathBuf> {
            panic!("mock narrator no-op must not call role_path");
        }

        fn read_file(&self, _path: &str) -> Result<String, AppError> {
            panic!("mock narrator no-op must not call read_file");
        }

        fn write_file(&self, _path: &str, _content: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call write_file");
        }

        fn remove_file(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call remove_file");
        }

        fn list_dir(&self, _path: &str) -> Result<Vec<PathBuf>, AppError> {
            panic!("mock narrator no-op must not call list_dir");
        }

        fn set_executable(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call set_executable");
        }

        fn file_exists(&self, _path: &str) -> bool {
            panic!("mock narrator no-op must not call file_exists");
        }

        fn is_dir(&self, _path: &str) -> bool {
            panic!("mock narrator no-op must not call is_dir");
        }

        fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
            panic!("mock narrator no-op must not call create_dir_all");
        }

        fn resolve_path(&self, _path: &str) -> PathBuf {
            panic!("mock narrator no-op must not call resolve_path");
        }

        fn canonicalize(&self, _path: &str) -> Result<PathBuf, AppError> {
            panic!("mock narrator no-op must not call canonicalize");
        }
    }

    #[test]
    fn narrator_mock_is_noop() {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Narrator, "jules-narrator-".to_string());
        let config = MockConfig {
            mock_tag: "mock-run-123".to_string(),
            branch_prefixes: prefixes,
            default_branch: "main".to_string(),
            jules_branch: "jules".to_string(),
            issue_labels: vec!["bugs".to_string()],
        };

        let output = execute_mock_narrator(
            Path::new(".jules"),
            &config,
            &MustNotTouchGit,
            &MustNotTouchGitHub,
            &DummyWorkspace,
        )
        .expect("mock narrator should succeed as no-op");

        assert_eq!(output.mock_branch, "");
        assert_eq!(output.mock_pr_number, 0);
        assert_eq!(output.mock_pr_url, "");
        assert_eq!(output.mock_tag, "mock-run-123");
    }
}
