use crate::app::commands::run::{self, RunOptions};
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer};
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};
use std::path::Path;

use super::options::{RunResults, WorkflowRunOptions};

mod decider;
mod implementer;
mod innovators;
mod integrator;
mod narrator;
mod observers;
mod planner;

/// Execute runs for a layer.
pub(crate) fn execute_layer<W, G, H>(
    store: &W,
    options: &WorkflowRunOptions,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
    G: Git,
    H: GitHub,
{
    let mut run_layer =
        |path: &Path, run_options: RunOptions, git_ref: &G, github_ref: &H, store_ref: &W| {
            run::execute(path, run_options, git_ref, github_ref, store_ref).map(|_| ())
        };

    execute_layer_with_runner(store, options, git, github, &mut run_layer)
}

fn execute_layer_with_runner<W, G, H, F>(
    store: &W,
    options: &WorkflowRunOptions,
    git: &G,
    github: &H,
    run_layer: &mut F,
) -> Result<RunResults, AppError>
where
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
    G: Git,
    H: GitHub,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let jules_path = store.jules_path();

    match options.layer {
        Layer::Narrator => narrator::execute(store, options, &jules_path, git, github, run_layer),
        Layer::Observers => observers::execute(store, options, &jules_path, git, github, run_layer),
        Layer::Decider => decider::execute(store, options, &jules_path, git, github, run_layer),
        Layer::Planner => planner::execute(store, options, &jules_path, git, github, run_layer),
        Layer::Implementer => {
            implementer::execute(store, options, &jules_path, git, github, run_layer)
        }
        Layer::Innovators => {
            innovators::execute(store, options, &jules_path, git, github, run_layer)
        }
        Layer::Integrator => {
            integrator::execute(store, options, &jules_path, git, github, run_layer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{GitHub, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
    use crate::testing::TestStore;

    struct NoopGit;

    impl Git for NoopGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            Ok("deadbeef".to_string())
        }

        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok("jules".to_string())
        }

        fn commit_exists(&self, _sha: &str) -> bool {
            true
        }

        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
            Ok("deadbeef".to_string())
        }

        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            Ok(false)
        }

        fn run_command(&self, _args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
            Ok(String::new())
        }

        fn checkout_branch(&self, _branch: &str, _create: bool) -> Result<(), AppError> {
            Ok(())
        }

        fn push_branch(&self, _branch: &str, _force: bool) -> Result<(), AppError> {
            Ok(())
        }

        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            Ok("deadbeef".to_string())
        }

        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    struct NoopGitHub;

    impl GitHub for NoopGitHub {
        fn create_pull_request(
            &self,
            head: &str,
            base: &str,
            _title: &str,
            _body: &str,
        ) -> Result<PullRequestInfo, AppError> {
            Ok(PullRequestInfo {
                number: 1,
                url: "https://example.com/pr/1".to_string(),
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
        ) -> Result<IssueInfo, AppError> {
            Ok(IssueInfo { number: 1, url: "https://example.com/issues/1".to_string() })
        }

        fn get_pr_detail(&self, _pr_number: u64) -> Result<PullRequestDetail, AppError> {
            Ok(PullRequestDetail {
                number: 1,
                head: "head".to_string(),
                base: "base".to_string(),
                is_draft: false,
                auto_merge_enabled: false,
            })
        }

        fn list_pr_comments(&self, _pr_number: u64) -> Result<Vec<PrComment>, AppError> {
            Ok(vec![])
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
            Ok(vec![])
        }
    }

    #[test]
    fn execute_layer_observers_reflects_enabled_roles_in_config() {
        let store = TestStore::new().with_exists(true).with_file(
            ".jlo/config.toml",
            r#"
[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[observers]
roles = [
  { name = "taxonomy", enabled = true },
  { name = "qa", enabled = false },
  { name = "consistency", enabled = true },
  { name = "cov", enabled = true },
]
"#,
        );
        let options = WorkflowRunOptions {
            layer: Layer::Observers,
            mock: true,
            mock_tag: Some("mock-test-001".to_string()),
            task: None,
        };
        let git = NoopGit;
        let github = NoopGitHub;

        let mut executed_roles: Vec<String> = Vec::new();
        let mut run_layer = |_path: &Path,
                             run_options: RunOptions,
                             _git: &NoopGit,
                             _gh: &NoopGitHub,
                             _store: &TestStore| {
            executed_roles.push(run_options.role.expect("role should be present"));
            Ok(())
        };

        let out =
            execute_layer_with_runner(&store, &options, &git, &github, &mut run_layer).unwrap();
        assert!(out.mock_pr_numbers.is_none());
        assert!(out.mock_branches.is_none());
        assert_eq!(
            executed_roles,
            vec!["taxonomy".to_string(), "consistency".to_string(), "cov".to_string()]
        );
    }
}
