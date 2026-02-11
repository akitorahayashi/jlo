use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::mock::identity::generate_mock_id;
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

// Template placeholder constants (must match src/assets/mock/innovator_idea.yml)
const TMPL_ID: &str = "mock01";
const TMPL_PERSONA: &str = "mock-persona";
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
/// Phase-driven semantics:
/// - `--phase creation` → create `idea.yml` (idea generation mock).
/// - `--phase refinement` → remove `idea.yml` (proposal creation + cleanup mock).
/// - Phase is required; omitting it is an error.
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
    let role = options.role.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Role (persona) is required for innovators".to_string())
    })?;

    let phase = options.phase.as_deref().ok_or_else(|| {
        AppError::MissingArgument(
            "--phase is required for innovators (creation or refinement)".to_string(),
        )
    })?;

    if phase != "creation" && phase != "refinement" {
        return Err(AppError::Validation(format!(
            "Invalid phase '{}': must be 'creation' or 'refinement'",
            phase
        )));
    }

    // Validate path components
    if !validate_safe_path_component(role) {
        return Err(AppError::Validation(format!(
            "Invalid role name '{}': must be alphanumeric with hyphens or underscores only",
            role
        )));
    }

    let room_dir = jules::innovator_persona_dir(jules_path, role);

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

    let is_creation = phase == "creation";

    println!("Mock innovators: phase={} for {}", phase, role);

    if is_creation {
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
            .replace(TMPL_DATE, &Utc::now().format("%Y-%m-%d").to_string())
            .replace(TMPL_TAG, &safe_tag);

        workspace.write_file(idea_path_str, &idea_content)?;
        let files: Vec<&Path> = vec![idea_path.as_path()];
        git.commit_files(
            &format!("[{}] innovator: mock creation (create idea)", config.mock_tag),
            &files,
        )?;
    } else {
        // Refinement phase mock: remove idea.yml (simulates proposal creation + cleanup)
        if workspace.file_exists(idea_path_str) {
            workspace.remove_file(idea_path_str)?;
            let files: Vec<&Path> = vec![idea_path.as_path()];
            git.commit_files(
                &format!("[{}] innovator: mock refinement (remove idea)", config.mock_tag),
                &files,
            )?;
        }
    }

    git.push_branch(&branch_name, false)?;

    // Create PR targeting jules branch
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[{}] Innovator {} {}", config.mock_tag, role, phase),
        &format!(
            "Mock innovator run for workflow validation.\n\n\
             Mock tag: `{}`\nPersona: `{}`\nPhase: {}",
            config.mock_tag, role, phase
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
    fn mock_innovator_creates_idea_with_creation_phase() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: Some("creation".to_string()),
        };

        let result =
            execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.mock_branch.starts_with("jules-innovator-"));
        assert_eq!(output.mock_pr_number, 42);

        // idea.yml should now exist
        let idea_path = jules_path.join("exchange/innovators/alice/idea.yml");
        assert!(workspace.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_removes_idea_with_refinement_phase() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        // Pre-populate idea.yml
        let idea_path = jules_path.join("exchange/innovators/alice/idea.yml");
        workspace.write_file(idea_path.to_str().unwrap(), "existing idea").unwrap();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: Some("refinement".to_string()),
        };

        let result =
            execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_ok());

        // idea.yml should be removed
        assert!(!workspace.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_creation_then_refinement_is_deterministic() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        let idea_path = jules_path.join("exchange/innovators/alice/idea.yml");

        // Creation phase: creates idea.yml
        let create_options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: Some("creation".to_string()),
        };
        let _ = execute_mock_innovators(
            &jules_path,
            &create_options,
            &config,
            &git,
            &github,
            &workspace,
        )
        .unwrap();
        assert!(workspace.file_exists(idea_path.to_str().unwrap()));

        // Refinement phase: removes idea.yml
        let refine_options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: Some("refinement".to_string()),
        };
        let _ = execute_mock_innovators(
            &jules_path,
            &refine_options,
            &config,
            &git,
            &github,
            &workspace,
        )
        .unwrap();
        assert!(!workspace.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_rejects_missing_phase() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: None,
        };

        let result =
            execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_err());
    }

    #[test]
    fn mock_innovator_rejects_invalid_phase() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: Some("invalid".to_string()),
        };

        let result =
            execute_mock_innovators(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_err());
    }
}
