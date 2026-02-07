use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::mock::identity::generate_mock_id;
use crate::domain::identities::validation::validate_safe_path_component;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

// Template placeholder constants (must match src/assets/mock/innovator_idea.yml)
const TMPL_ID: &str = "mock01";
const TMPL_PERSONA: &str = "mock-persona";
const TMPL_WORKSTREAM: &str = "mock-workstream";
const TMPL_DATE: &str = "2026-02-05";
const TMPL_TAG: &str = "test-tag";

/// Sanitize a value for safe embedding in YAML scalar fields.
/// Strips characters that could break YAML structure.
fn sanitize_yaml_value(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | ':' | '#' | '\'' | '"' | '{' | '}' | '[' | ']'))
        .collect()
}

/// Execute mock innovators.
///
/// Toggle semantics:
/// - If `idea.yml` is absent → create `idea.yml` (creation phase mock).
/// - If `idea.yml` is present → remove `idea.yml` (refinement phase mock).
/// - Two invocations in one cycle leave a clean room state.
pub fn execute_mock_innovators<G, H, W>(
    jules_path: &Path,
    options: &RunOptions,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore,
{
    let workstream = options.workstream.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Workstream is required for innovators".to_string())
    })?;

    let role = options.role.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Role (persona) is required for innovators".to_string())
    })?;

    // Validate path components
    if !validate_safe_path_component(workstream) {
        return Err(AppError::Validation(format!(
            "Invalid workstream name '{}': must be alphanumeric with hyphens or underscores only",
            workstream
        )));
    }
    if !validate_safe_path_component(role) {
        return Err(AppError::Validation(format!(
            "Invalid role name '{}': must be alphanumeric with hyphens or underscores only",
            role
        )));
    }

    let room_dir = jules_path
        .join("workstreams")
        .join(workstream)
        .join("exchange")
        .join("innovators")
        .join(role);

    let idea_path = room_dir.join("idea.yml");
    let idea_path_str = idea_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid idea.yml path".to_string()))?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Innovators, &timestamp)?;

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Ensure room directory exists
    let room_dir_str =
        room_dir.to_str().ok_or_else(|| AppError::Validation("Invalid room path".to_string()))?;
    workspace.create_dir_all(room_dir_str)?;

    // Check idea.yml existence after checkout to avoid TOCTOU race
    let idea_exists = workspace.file_exists(idea_path_str);

    println!(
        "Mock innovators: {} idea.yml for {}/{}",
        if idea_exists { "removing" } else { "creating" },
        workstream,
        role
    );

    if idea_exists {
        // Refinement phase mock: remove idea.yml (simulates proposal creation + cleanup)
        workspace.remove_file(idea_path_str)?;
        let files: Vec<&Path> = vec![idea_path.as_path()];
        git.commit_files(
            &format!("[{}] innovator: mock refinement (remove idea)", config.mock_tag),
            &files,
        )?;
    } else {
        // Creation phase mock: create idea.yml from template
        let mock_idea_template = super::MOCK_ASSETS
            .get_file("innovator_idea.yml")
            .ok_or_else(|| {
                AppError::InternalError("Mock asset missing: innovator_idea.yml".to_string())
            })?
            .contents_utf8()
            .ok_or_else(|| {
                AppError::InternalError("Invalid UTF-8 in innovator_idea.yml".to_string())
            })?;

        let idea_id = generate_mock_id();
        let safe_tag = sanitize_yaml_value(&config.mock_tag);
        let idea_content = mock_idea_template
            .replace(TMPL_ID, &idea_id)
            .replace(TMPL_PERSONA, role)
            .replace(TMPL_WORKSTREAM, workstream)
            .replace(TMPL_DATE, &Utc::now().format("%Y-%m-%d").to_string())
            .replace(TMPL_TAG, &safe_tag);

        workspace.write_file(idea_path_str, &idea_content)?;
        let files: Vec<&Path> = vec![idea_path.as_path()];
        git.commit_files(
            &format!("[{}] innovator: mock creation (create idea)", config.mock_tag),
            &files,
        )?;
    }

    git.push_branch(&branch_name, false)?;

    // Create PR targeting jules branch
    let action = if idea_exists { "refinement" } else { "creation" };
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[{}] Innovator {} {}", config.mock_tag, role, action),
        &format!(
            "Mock innovator run for workflow validation.\n\n\
             Mock tag: `{}`\nWorkstream: `{}`\nPersona: `{}`\nPhase: {}",
            config.mock_tag, workstream, role, action
        ),
    )?;

    println!("Mock innovators: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockWorkspaceStore;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct FakeGit {
        branches_created: Mutex<Vec<String>>,
    }

    impl FakeGit {
        fn new() -> Self {
            Self { branches_created: Mutex::new(vec![]) }
        }
    }

    impl GitPort for FakeGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            Ok("abc123".into())
        }
        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok("jules".into())
        }
        fn commit_exists(&self, _sha: &str) -> bool {
            true
        }
        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
            Ok("parent".into())
        }
        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            Ok(false)
        }
        fn count_commits(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<u32, AppError> {
            Ok(0)
        }
        fn collect_commits(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
            _limit: usize,
        ) -> Result<Vec<crate::ports::CommitInfo>, AppError> {
            Ok(vec![])
        }
        fn get_diffstat(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<crate::ports::DiffStat, AppError> {
            Ok(Default::default())
        }
        fn run_command(&self, _args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
            Ok(String::new())
        }
        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn checkout_branch(&self, name: &str, create: bool) -> Result<(), AppError> {
            if create {
                self.branches_created.lock().unwrap().push(name.to_string());
            }
            Ok(())
        }
        fn commit_files(&self, _msg: &str, _files: &[&Path]) -> Result<String, AppError> {
            Ok("fake-sha".into())
        }
        fn push_branch(&self, _name: &str, _force: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    struct FakeGitHub;

    impl GitHubPort for FakeGitHub {
        fn dispatch_workflow(
            &self,
            _workflow_name: &str,
            _inputs: &[(&str, &str)],
        ) -> Result<(), AppError> {
            Ok(())
        }
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
    }

    fn make_config() -> MockConfig {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Innovators, "jules-innovator-".to_string());
        MockConfig {
            mock_tag: "mock-test-001".to_string(),
            branch_prefixes: prefixes,
            default_branch: "main".to_string(),
            jules_branch: "jules".to_string(),
            issue_labels: vec![],
        }
    }

    #[test]
    fn mock_innovator_creates_idea_when_absent() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            workstream: Some("generic".to_string()),
            prompt_preview: false,
            branch: None,
            issue: None,
            mock: true,
        };

        let result =
            execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.mock_branch.starts_with("jules-innovator-"));
        assert_eq!(output.mock_pr_number, 42);

        // idea.yml should now exist
        let idea_path = jules_path.join("workstreams/generic/exchange/innovators/alice/idea.yml");
        assert!(workspace.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_removes_idea_when_present() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        // Pre-populate idea.yml
        let idea_path = jules_path.join("workstreams/generic/exchange/innovators/alice/idea.yml");
        workspace.write_file(idea_path.to_str().unwrap(), "existing idea").unwrap();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            workstream: Some("generic".to_string()),
            prompt_preview: false,
            branch: None,
            issue: None,
            mock: true,
        };

        let result =
            execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_ok());

        // idea.yml should be removed
        assert!(!workspace.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_double_toggle_leaves_clean_state() {
        // Note: Both invocations may produce the same branch name when run
        // within the same second. This is acceptable because FakeGit does
        // not enforce branch uniqueness, and real execution is serialized
        // by the workflow scheduler with distinct timestamps.
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            workstream: Some("generic".to_string()),
            prompt_preview: false,
            branch: None,
            issue: None,
            mock: true,
        };

        let idea_path = jules_path.join("workstreams/generic/exchange/innovators/alice/idea.yml");

        // First invocation: creates idea.yml
        let _ = execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace)
            .unwrap();
        assert!(workspace.file_exists(idea_path.to_str().unwrap()));

        // Second invocation: removes idea.yml
        let _ = execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace)
            .unwrap();
        assert!(!workspace.file_exists(idea_path.to_str().unwrap()));
    }
}
