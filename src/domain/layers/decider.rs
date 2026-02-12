use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::domain::configuration::loader::detect_repository_source;
use crate::domain::configuration::mock_loader::load_mock_config;
use crate::domain::layers::mock_utils::{MOCK_ASSETS, generate_mock_id};
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, IoErrorKind, Layer, MockConfig, MockOutput, RunConfig, RunOptions};
use crate::ports::{AutomationMode, GitHubPort, GitPort, SessionRequest, WorkspaceStore};

use super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct DeciderLayer;

impl<W> LayerStrategy<W> for DeciderLayer
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn GitPort,
        github: &dyn GitHubPort,
        workspace: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if options.mock {
            let mock_config = load_mock_config(jules_path, options, workspace)?;
            let output = execute_mock(jules_path, options, &mock_config, git, github, workspace)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::mock_utils::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::mock_utils::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec!["decider".to_string()],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            options.prompt_preview,
            options.branch.as_deref(),
            config,
            git,
            workspace,
            client_factory,
        )
    }
}

fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    config: &RunConfig,
    git: &G,
    workspace: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
    G: GitPort + ?Sized,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_branch.clone());

    if prompt_preview {
        println!("=== Prompt Preview: Decider ===");
        println!("Starting branch: {}\n", starting_branch);

        let prompt = assemble_decider_prompt(jules_path, workspace)?;
        println!("  Assembled prompt: {} chars", prompt.len());

        println!("\nWould dispatch workflow");
        return Ok(RunResult {
            roles: vec!["decider".to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let source = detect_repository_source(git)?;
    let client = client_factory.create()?;

    let prompt = assemble_decider_prompt(jules_path, workspace)?;

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch: starting_branch.to_string(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    println!("Executing: decider...");
    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(RunResult {
        roles: vec!["decider".to_string()],
        prompt_preview: false,
        sessions: vec![response.session_id],
        cleanup_requirement: None,
    })
}

fn assemble_decider_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    workspace: &W,
) -> Result<String, AppError> {
    assemble_prompt(jules_path, Layer::Decider, &PromptContext::new(), workspace)
        .map(|p: AssembledPrompt| p.content)
        .map_err(|e| AppError::InternalError(e.to_string()))
}

fn execute_mock<G, H, W>(
    jules_path: &Path,
    _options: &RunOptions,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort + ?Sized,
    H: GitHubPort + ?Sized,
    W: WorkspaceStore,
{
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Decider, &timestamp)?;

    println!("Mock decider: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Find and process pending events
    let pending_dir = jules::events_pending_dir(jules_path);
    let decided_dir = jules::events_decided_dir(jules_path);
    let requirements_dir = jules::requirements_dir(jules_path);

    // Ensure directories exist.
    workspace.create_dir_all(path_to_str(&decided_dir, "Invalid decided events path")?)?;
    workspace.create_dir_all(path_to_str(&requirements_dir, "Invalid requirements path")?)?;

    // Create two mock requirements: one for planner, one for implementer
    let label = config.issue_labels.first().cloned().ok_or_else(|| {
        AppError::Validation("No issue labels available for mock decider".to_string())
    })?;
    let mock_issue_template = MOCK_ASSETS
        .get_file("decider_requirement.yml")
        .ok_or_else(|| {
            AppError::InternalError("Mock asset missing: decider_requirement.yml".to_string())
        })?
        .contents_utf8()
        .ok_or_else(|| {
            AppError::InternalError("Invalid UTF-8 in decider_requirement.yml".to_string())
        })?;

    // Move any mock pending events to decided first
    let mut moved_src_files: Vec<PathBuf> = Vec::new();
    for path in list_mock_tagged_files(workspace, &pending_dir, &config.mock_tag)? {
        let source = path_to_str(&path, "Invalid pending event path")?;
        let content = workspace.read_file(source)?;
        let dest = decided_dir.join(path.file_name().ok_or_else(|| {
            AppError::Validation(format!("Pending event missing filename: {}", path.display()))
        })?);
        workspace.write_file(path_to_str(&dest, "Invalid decided event path")?, &content)?;
        workspace.remove_file(source)?;
        moved_src_files.push(path);
    }

    let decided_mock_files = list_mock_decided_files(workspace, &decided_dir, &config.mock_tag)?;
    let source_event_ids: Vec<String> = decided_mock_files
        .iter()
        .filter_map(|path| mock_event_id_from_path(path, &config.mock_tag))
        .collect();

    if source_event_ids.len() < 2 {
        return Err(AppError::Validation(format!(
            "Mock decider requires at least 2 decided events for tag '{}', found {}",
            config.mock_tag,
            source_event_ids.len()
        )));
    }

    let planner_source_event_ids = vec![source_event_ids[0].clone()];
    let impl_source_event_ids: Vec<String> = source_event_ids[1..].to_vec();

    // Requirement 1: requires deep analysis (for planner)
    let planner_issue_id = generate_mock_id();
    let planner_issue_file = requirements_dir.join(format!("mock-planner-{}.yml", config.mock_tag));

    let mut planner_issue_yaml: serde_yaml::Value = serde_yaml::from_str(mock_issue_template)
        .map_err(|e| {
            AppError::InternalError(format!("Failed to parse mock issue template: {}", e))
        })?;

    if let Some(mapping) = planner_issue_yaml.as_mapping_mut() {
        mapping.insert("id".into(), planner_issue_id.clone().into());
        mapping.insert("label".into(), label.clone().into());
        mapping.insert(
            "summary".into(),
            format!(
                "This is a mock requirement created by jlo --mock for workflow-scaffold validation. Mock tag: {}",
                config.mock_tag
            )
            .into(),
        );
        let src_events = mapping
            .entry("source_events".into())
            .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));
        if let Some(seq) = src_events.as_sequence_mut() {
            seq.clear();
            for event_id in &planner_source_event_ids {
                seq.push(event_id.clone().into());
            }
        }

        mapping.insert("title".into(), "Mock requirement requiring deep analysis".into());
        mapping.insert("priority".into(), "high".into());
        mapping.insert("requires_deep_analysis".into(), true.into());
        mapping.insert(
            "deep_analysis_reason".into(),
            "Mock requirement requires architectural analysis-for-workflow-validation".into(),
        );
    }

    workspace.write_file(
        planner_issue_file
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &serde_yaml::to_string(&planner_issue_yaml).map_err(|err| {
            AppError::InternalError(format!(
                "Failed to serialize planner requirement YAML: {}",
                err
            ))
        })?,
    )?;

    // Requirement 2: ready for implementer
    let impl_issue_id = generate_mock_id();
    let impl_issue_file = requirements_dir.join(format!("mock-impl-{}.yml", config.mock_tag));

    let mut impl_issue_yaml: serde_yaml::Value = serde_yaml::from_str(mock_issue_template)
        .map_err(|e| {
            AppError::InternalError(format!("Failed to parse mock issue template: {}", e))
        })?;

    if let Some(mapping) = impl_issue_yaml.as_mapping_mut() {
        mapping.insert("id".into(), impl_issue_id.clone().into());
        mapping.insert("label".into(), label.clone().into());
        mapping.insert(
            "summary".into(),
            format!(
                "This is a mock requirement created by jlo --mock for workflow-scaffold validation. Mock tag: {}",
                config.mock_tag
            )
            .into(),
        );
        let src_events = mapping
            .entry("source_events".into())
            .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));
        if let Some(seq) = src_events.as_sequence_mut() {
            seq.clear();
            for event_id in &impl_source_event_ids {
                seq.push(event_id.clone().into());
            }
        }

        mapping.insert("title".into(), "Mock requirement ready for implementation".into());
        mapping.insert("requires_deep_analysis".into(), false.into());
    }

    workspace.write_file(
        impl_issue_file.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &serde_yaml::to_string(&impl_issue_yaml).map_err(|err| {
            AppError::InternalError(format!(
                "Failed to serialize implementer requirement YAML: {}",
                err
            ))
        })?,
    )?;

    // Ensure all tag-matched decided events have issue_id.
    // Each event belongs to exactly one requirement.
    let planner_event_set: HashSet<&str> =
        planner_source_event_ids.iter().map(|event_id| event_id.as_str()).collect();
    for decided_file in &decided_mock_files {
        if let Some(event_id) = mock_event_id_from_path(decided_file, &config.mock_tag) {
            let assigned_issue_id = if planner_event_set.contains(event_id.as_str()) {
                &planner_issue_id
            } else {
                &impl_issue_id
            };

            let decided_file_str = match decided_file.to_str() {
                Some(path) => path,
                None => {
                    println!(
                        "::warning::Invalid decided event file path (non UTF-8): {}",
                        decided_file.display()
                    );
                    continue;
                }
            };

            let content = match workspace.read_file(decided_file_str) {
                Ok(content) => content,
                Err(err) => {
                    println!(
                        "::warning::Failed to read decided event file {}: {}",
                        decided_file.display(),
                        err
                    );
                    continue;
                }
            };

            let mut yaml_value: serde_yaml::Value = match serde_yaml::from_str(&content) {
                Ok(value) => value,
                Err(err) => {
                    println!(
                        "::warning::Failed to parse decided event file {} as YAML: {}",
                        decided_file.display(),
                        err
                    );
                    continue;
                }
            };

            let Some(mapping) = yaml_value.as_mapping_mut() else {
                println!(
                    "::warning::Decided event file is not a YAML mapping: {}",
                    decided_file.display()
                );
                continue;
            };

            mapping.insert(
                serde_yaml::Value::String("issue_id".to_string()),
                serde_yaml::Value::String(assigned_issue_id.to_string()),
            );

            let updated_content = match serde_yaml::to_string(&yaml_value) {
                Ok(value) => value,
                Err(err) => {
                    println!(
                        "::warning::Failed to render decided event YAML {}: {}",
                        decided_file.display(),
                        err
                    );
                    continue;
                }
            };

            if let Err(err) = workspace.write_file(decided_file_str, &updated_content) {
                println!(
                    "::warning::Failed to write decided event file {}: {}",
                    decided_file.display(),
                    err
                );
            }
        }
    }

    // Commit and push (include moved/deleted files and decided updates)
    let mut files: Vec<&Path> = vec![planner_issue_file.as_path(), impl_issue_file.as_path()];
    for f in &decided_mock_files {
        files.push(f.as_path());
    }
    for f in &moved_src_files {
        files.push(f.as_path());
    }
    git.commit_files(&format!("[{}] decider: mock requirements", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[{}] Decider triage", config.mock_tag),
        &format!("Mock decider run for workflow validation.\n\nMock tag: `{}`\n\nCreated requirements:\n- `{}` (requires analysis)\n- `{}` (ready for impl)",
            config.mock_tag, planner_issue_id, impl_issue_id),
    )?;

    println!("Mock decider: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}

fn mock_event_id_from_path(path: &Path, mock_tag: &str) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let prefix = format!("mock-{}-", mock_tag);
    file_name.strip_prefix(&prefix)?.strip_suffix(".yml").map(ToString::to_string)
}

fn list_mock_decided_files<W: WorkspaceStore>(
    workspace: &W,
    decided_dir: &Path,
    mock_tag: &str,
) -> Result<Vec<PathBuf>, AppError> {
    list_mock_tagged_files(workspace, decided_dir, mock_tag)
}

fn list_mock_tagged_files<W: WorkspaceStore>(
    workspace: &W,
    dir: &Path,
    mock_tag: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let dir_str = path_to_str(dir, "Invalid directory path")?;
    let entries = match workspace.list_dir(dir_str) {
        Ok(entries) => entries,
        Err(AppError::Io { kind: IoErrorKind::NotFound, .. }) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let mut files: Vec<PathBuf> = entries
        .into_iter()
        .filter(|path| !workspace.is_dir(&path.to_string_lossy()))
        .filter(|path| mock_event_id_from_path(path, mock_tag).is_some())
        .collect();

    files.sort();
    Ok(files)
}

fn path_to_str<'a>(path: &'a Path, err_prefix: &str) -> Result<&'a str, AppError> {
    path.to_str().ok_or_else(|| AppError::Validation(format!("{}: {}", err_prefix, path.display())))
}

#[cfg(test)]
mod tests {
    use super::{list_mock_decided_files, mock_event_id_from_path};
    use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_mock_event_id_from_path() {
        let mock_tag = "mock-run-123";
        let valid_path = std::path::Path::new("mock-mock-run-123-a1b2c3.yml");
        let invalid_path = std::path::Path::new("mock-other-tag-a1b2c3.yml");

        assert_eq!(mock_event_id_from_path(valid_path, mock_tag), Some("a1b2c3".to_string()));
        assert_eq!(mock_event_id_from_path(invalid_path, mock_tag), None);
    }

    #[test]
    fn lists_only_tagged_decided_files_in_sorted_order() {
        let dir = tempdir().expect("tempdir");
        let decided_dir = dir.path().join("decided");
        fs::create_dir_all(&decided_dir).expect("mkdir");

        fs::write(decided_dir.join("mock-mock-run-123-bbbbbb.yml"), "id: bbbbbb\n").expect("write");
        fs::write(decided_dir.join("mock-mock-run-123-aaaaaa.yml"), "id: aaaaaa\n").expect("write");
        fs::write(decided_dir.join("mock-other-run-cccccc.yml"), "id: cccccc\n").expect("write");
        fs::write(decided_dir.join("notes.txt"), "ignored\n").expect("write");

        let workspace = FilesystemWorkspaceStore::new(dir.path().to_path_buf());
        let files =
            list_mock_decided_files(&workspace, &decided_dir, "mock-run-123").expect("list");

        assert_eq!(files.len(), 2);
        assert!(files[0].to_string_lossy().ends_with("mock-mock-run-123-aaaaaa.yml"));
        assert!(files[1].to_string_lossy().ends_with("mock-mock-run-123-bbbbbb.yml"));
    }

    use super::*;
    use crate::testing::MockWorkspaceStore;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct FakeGit {
        committed_files: Mutex<Vec<PathBuf>>,
    }

    impl FakeGit {
        fn new() -> Self {
            Self { committed_files: Mutex::new(vec![]) }
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
        fn checkout_branch(&self, _name: &str, _create: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn push_branch(&self, _name: &str, _force: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(true)
        }
        fn commit_files(&self, _msg: &str, files: &[&Path]) -> Result<String, AppError> {
            let mut committed = self.committed_files.lock().unwrap();
            for f in files {
                committed.push(f.to_path_buf());
            }
            Ok("fake-sha".into())
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
                number: 101,
                url: "https://example.com/pr/101".into(),
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
                number: 101,
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
        prefixes.insert(Layer::Decider, "jules-decider-".to_string());
        MockConfig {
            mock_tag: "mock-test-decider".to_string(),
            branch_prefixes: prefixes,
            default_branch: "main".to_string(),
            jules_branch: "jules".to_string(),
            issue_labels: vec!["bugs".to_string()],
        }
    }

    #[test]
    fn mock_decider_processes_events_and_creates_requirements() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        // Create 2 mock pending events
        workspace
            .write_file(
                ".jules/exchange/events/pending/mock-mock-test-decider-event1.yml",
                "id: event1\nsummary: s1",
            )
            .unwrap();
        workspace
            .write_file(
                ".jules/exchange/events/pending/mock-mock-test-decider-event2.yml",
                "id: event2\nsummary: s2",
            )
            .unwrap();

        let options = RunOptions {
            layer: Layer::Decider,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: None,
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.mock_branch.starts_with("jules-decider-"));
        assert_eq!(output.mock_pr_number, 101);

        // Verify requirements created (planner and implementer)
        let req_dir = ".jules/exchange/requirements";
        let req_files = workspace.list_dir(req_dir).unwrap();
        let planner_req = req_files
            .iter()
            .find(|p| p.to_string_lossy().contains("mock-planner-mock-test-decider"))
            .expect("planner req missing");
        let impl_req = req_files
            .iter()
            .find(|p| p.to_string_lossy().contains("mock-impl-mock-test-decider"))
            .expect("implementer req missing");

        assert!(workspace.file_exists(&planner_req.to_string_lossy()));
        assert!(workspace.file_exists(&impl_req.to_string_lossy()));

        // Verify events moved to decided
        assert!(
            !workspace
                .file_exists(".jules/exchange/events/pending/mock-mock-test-decider-event1.yml")
        );
        assert!(
            workspace
                .file_exists(".jules/exchange/events/decided/mock-mock-test-decider-event1.yml")
        );
    }

    #[test]
    fn mock_decider_fails_with_insufficient_events() {
        let jules_path = PathBuf::from(".jules");
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub;
        let config = make_config();

        // Only 1 event
        workspace
            .write_file(
                ".jules/exchange/events/pending/mock-mock-test-decider-event1.yml",
                "id: event1",
            )
            .unwrap();

        let options = RunOptions {
            layer: Layer::Decider,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            phase: None,
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &workspace);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(AppError::Validation(msg)) if msg.contains("requires at least 2 decided events"))
        );
    }
}
