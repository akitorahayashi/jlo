use crate::adapters::schedule_filesystem::load_schedule;
use crate::app::commands::run::{self, RunOptions};
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};
use std::path::Path;

use super::issue_routing::find_requirements;
use super::options::{RunResults, WorkflowRunOptions};

/// Execute runs for a layer.
pub(crate) fn execute_layer<W, G, H>(
    store: &W,
    options: &WorkflowRunOptions,
    git: &G,
    github: &H,
) -> Result<RunResults, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
    G: GitPort,
    H: GitHubPort,
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
    W: WorkspaceStore + Clone + Send + Sync + 'static,
    G: GitPort,
    H: GitHubPort,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let jules_path = store.jules_path();

    match options.layer {
        Layer::Narrator => execute_narrator(store, options, &jules_path, git, github, run_layer),
        Layer::Observers => execute_multi_role(store, options, &jules_path, git, github, run_layer),
        Layer::Decider => execute_decider(store, options, &jules_path, git, github, run_layer),
        Layer::Planner => {
            execute_requirement_layer(store, options, &jules_path, git, github, run_layer)
        }
        Layer::Implementer => {
            execute_requirement_layer(store, options, &jules_path, git, github, run_layer)
        }
        Layer::Innovators => {
            execute_multi_role(store, options, &jules_path, git, github, run_layer)
        }
    }
}

/// Execute narrator.
fn execute_narrator<W, G, H, F>(
    store: &W,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    run_layer: &mut F,
) -> Result<RunResults, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
    G: GitPort,
    H: GitHubPort,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let run_options = RunOptions {
        layer: Layer::Narrator,
        role: None,
        prompt_preview: false,
        branch: None,
        requirement: None,
        mock: options.mock,
        phase: None,
    };

    eprintln!("Executing: narrator{}", if options.mock { " (mock)" } else { "" });
    run_layer(jules_path, run_options, git, github, store)?;

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute decider (single-role, gated by pending events).
fn execute_decider<W, G, H, F>(
    store: &W,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    run_layer: &mut F,
) -> Result<RunResults, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
    G: GitPort,
    H: GitHubPort,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    // Gate: only proceed if pending events exist (or mock mode)
    if !options.mock && !has_pending_events(jules_path)? {
        eprintln!("No pending events, skipping decider");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    let run_options = RunOptions {
        layer: Layer::Decider,
        role: None,
        prompt_preview: false,
        branch: None,
        requirement: None,
        mock: options.mock,
        phase: None,
    };

    eprintln!("Executing: decider{}", if options.mock { " (mock)" } else { "" });
    run_layer(jules_path, run_options, git, github, store)?;

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Check if the pending events directory contains any .yml files.
fn has_pending_events(jules_path: &Path) -> Result<bool, AppError> {
    let pending_dir = jules::exchange_dir(jules_path).join("events/pending");
    if !pending_dir.exists() {
        return Ok(false);
    }
    let entries = std::fs::read_dir(&pending_dir)?;
    for entry in entries {
        let entry = entry?;
        if entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "yml") {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Execute multi-role layer (observers, innovators).
fn execute_multi_role<W, G, H, F>(
    store: &W,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    run_layer: &mut F,
) -> Result<RunResults, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
    G: GitPort,
    H: GitHubPort,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    // Load root schedule
    let schedule = load_schedule(store)?;

    if !schedule.enabled {
        eprintln!("Schedule is disabled, skipping");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    // Get enabled roles for the layer
    let roles = match options.layer {
        Layer::Observers => schedule.observers.enabled_roles(),
        Layer::Innovators => {
            schedule.innovators.as_ref().map(|l| l.enabled_roles()).unwrap_or_default()
        }
        _ => {
            return Err(AppError::Validation("Invalid layer for multi-role execution".to_string()));
        }
    };

    if roles.is_empty() {
        eprintln!("No enabled {} roles", options.layer.dir_name());
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    // Execute each role
    for role in roles {
        let run_options = RunOptions {
            layer: options.layer,
            role: Some(role.as_str().to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: options.mock,
            phase: options.phase.clone(),
        };

        eprintln!("Executing: {} --role {}{}", options.layer.dir_name(), role, mock_suffix);
        run_layer(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

/// Execute requirement-based layers (planner, implementer).
fn execute_requirement_layer<W, G, H, F>(
    store: &W,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    run_layer: &mut F,
) -> Result<RunResults, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
    G: GitPort,
    H: GitHubPort,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };

    let requirements = find_requirements(store, options.layer)?;

    if requirements.is_empty() {
        eprintln!("No requirements found for {}", options.layer.dir_name());
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    for requirement_path in requirements {
        let run_options = RunOptions {
            layer: options.layer,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: Some(requirement_path.clone()),
            mock: options.mock,
            phase: None,
        };

        eprintln!(
            "Executing: {} {}{}",
            options.layer.dir_name(),
            requirement_path.display(),
            mock_suffix
        );
        run_layer(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{GitHubPort, IssueInfo, PrComment, PullRequestDetail, PullRequestInfo};
    use crate::testing::MockWorkspaceStore;

    struct NoopGit;

    impl GitPort for NoopGit {
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

        fn has_changes(&self, _from: &str, _to: &str, _pathspec: &[&str]) -> Result<bool, AppError> {
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

    impl GitHubPort for NoopGitHub {
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

        fn create_issue(&self, _title: &str, _body: &str, _labels: &[&str]) -> Result<IssueInfo, AppError> {
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
    fn execute_layer_observers_reflects_enabled_roles_in_scheduled_toml() {
        let store = MockWorkspaceStore::new().with_exists(true).with_file(
            ".jlo/scheduled.toml",
            r#"
version = 1
enabled = true

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
            phase: None,
        };
        let git = NoopGit;
        let github = NoopGitHub;

        let mut executed_roles: Vec<String> = Vec::new();
        let mut run_layer =
            |_path: &Path, run_options: RunOptions, _git: &NoopGit, _gh: &NoopGitHub, _store: &MockWorkspaceStore| {
                executed_roles.push(run_options.role.expect("role should be present"));
                Ok(())
            };

        let out = execute_layer_with_runner(&store, &options, &git, &github, &mut run_layer).unwrap();
        assert!(out.mock_pr_numbers.is_none());
        assert!(out.mock_branches.is_none());
        assert_eq!(
            executed_roles,
            vec!["taxonomy".to_string(), "consistency".to_string(), "cov".to_string()]
        );
    }
}
