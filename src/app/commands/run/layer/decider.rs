use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::super::mock::mock_execution::{
    MOCK_ASSETS, MockExecutionService, generate_mock_id, list_mock_tagged_files,
    mock_event_id_from_path,
};
use crate::app::commands::run::RunRuntimeOptions;
use crate::app::commands::run::input::{detect_repository_source, load_mock_config};
use crate::domain::layers::execute::starting_branch::resolve_starting_branch;
use crate::domain::prompt_assemble::{
    AssembledPrompt, PromptAssetLoader, PromptContext, assemble_prompt,
};
use crate::domain::{
    AppError, ConfigError, ControlPlaneConfig, Layer, MockConfig, MockOutput, RunOptions,
};
use crate::ports::{
    AutomationMode, Git, GitHub, JloStore, JulesStore, RepositoryFilesystem, SessionRequest,
};

use super::super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct DeciderLayer;

impl<W> LayerStrategy<W> for DeciderLayer
where
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        _target: &RunOptions,
        runtime: &RunRuntimeOptions,
        config: &ControlPlaneConfig,
        git: &dyn Git,
        github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if runtime.mock {
            let mock_config = load_mock_config(jules_path, repository)?;
            let _output = execute_mock(jules_path, &mock_config, git, github, repository)?;
            return Ok(RunResult {
                roles: vec!["decider".to_string()],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            runtime.prompt_preview,
            runtime.branch.as_deref(),
            config,
            git,
            repository,
            client_factory,
        )
    }
}

fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    config: &ControlPlaneConfig,
    git: &G,
    repository: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
    G: Git + ?Sized,
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
{
    let starting_branch = resolve_starting_branch(Layer::Decider, config, branch);

    if prompt_preview {
        println!("=== Prompt Preview: Decider ===");
        println!("Starting branch: {}\n", starting_branch);

        let prompt = assemble_decider_prompt(jules_path, repository)?;
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

    let prompt = assemble_decider_prompt(jules_path, repository)?;

    let request = SessionRequest {
        prompt,
        source: source.to_string(),
        starting_branch,
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

fn assemble_decider_prompt<
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
>(
    jules_path: &Path,
    repository: &W,
) -> Result<String, AppError> {
    assemble_prompt(
        jules_path,
        Layer::Decider,
        &PromptContext::new(),
        repository,
        crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset,
    )
    .map(|p: AssembledPrompt| p.content)
    .map_err(|e| AppError::InternalError(e.to_string()))
}

fn execute_mock<G, H, W>(
    jules_path: &Path,
    config: &MockConfig,
    git: &G,
    github: &H,
    repository: &W,
) -> Result<MockOutput, AppError>
where
    G: Git + ?Sized,
    H: GitHub + ?Sized,
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
{
    if !crate::domain::validation::validate_identifier(&config.mock_tag, false) {
        return Err(ConfigError::Invalid(format!(
            "mock_tag '{}' must be a safe path component (letters, numbers, '-' or '_')",
            config.mock_tag
        ))
        .into());
    }

    let service = MockExecutionService::new(jules_path, config, git, github, repository);

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Decider, &timestamp)?;

    println!("Mock decider: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    service.fetch_and_checkout_base(&config.jules_worker_branch)?;
    service.checkout_new_branch(&branch_name)?;

    // Find and process pending events
    let pending_dir = crate::domain::exchange::events::paths::events_pending_dir(jules_path);
    let decided_dir = crate::domain::exchange::events::paths::events_decided_dir(jules_path);
    let requirements_dir =
        crate::domain::exchange::requirements::paths::requirements_dir(jules_path);

    // Ensure directories exist.
    repository.create_dir_all(
        decided_dir.to_str().ok_or_else(|| AppError::InvalidPath("Invalid decided dir".into()))?,
    )?;
    repository.create_dir_all(
        requirements_dir
            .to_str()
            .ok_or_else(|| AppError::InvalidPath("Invalid requirements dir".into()))?,
    )?;

    // Create two mock requirements: one for planner, one for implementer
    let label = config.issue_labels.first().cloned().ok_or_else(|| {
        ConfigError::Invalid("No requirement labels available for mock decider".to_string())
    })?;
    let mock_requirement_template = MOCK_ASSETS
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
    for path in list_mock_tagged_files(repository, &pending_dir, &config.mock_tag)? {
        let source = path
            .to_str()
            .ok_or_else(|| AppError::InvalidPath("Invalid pending event path".into()))?;
        let content = repository.read_file(source)?;
        let dest = decided_dir.join(path.file_name().ok_or_else(|| {
            AppError::InvalidPath(format!("Pending event missing filename: {}", path.display()))
        })?);
        repository.write_file(
            dest.to_str()
                .ok_or_else(|| AppError::InvalidPath("Invalid decided event path".into()))?,
            &content,
        )?;
        repository.remove_file(source)?;
        moved_src_files.push(path);
    }

    let decided_mock_files = list_mock_tagged_files(repository, &decided_dir, &config.mock_tag)?;
    let source_event_ids: Vec<String> = decided_mock_files
        .iter()
        .filter_map(|path| mock_event_id_from_path(path, &config.mock_tag))
        .collect();

    if source_event_ids.len() < 2 {
        return Err(ConfigError::Invalid(format!(
            "Mock decider requires at least 2 decided events for tag '{}', found {}",
            config.mock_tag,
            source_event_ids.len()
        ))
        .into());
    }

    let planner_source_event_ids = vec![source_event_ids[0].clone()];
    let impl_source_event_ids: Vec<String> = source_event_ids[1..].to_vec();

    // Requirement 1: not implementation-ready (routes to planner)
    let planner_requirement_id = generate_mock_id();
    let planner_requirement_file =
        requirements_dir.join(format!("planner-{}.yml", config.mock_tag));

    let mut planner_requirement_yaml: serde_yaml::Value =
        serde_yaml::from_str(mock_requirement_template).map_err(|e| {
            AppError::InternalError(format!("Failed to parse mock requirement template: {}", e))
        })?;

    if let Some(mapping) = planner_requirement_yaml.as_mapping_mut() {
        mapping.insert("id".into(), planner_requirement_id.clone().into());
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

        mapping.insert("title".into(), "Mock requirement requiring planner detailization".into());
        mapping.insert("priority".into(), "high".into());
        mapping.insert("implementation_ready".into(), false.into());
        mapping.insert(
            "planner_request_reason".into(),
            "Mock requirement needs planner detailization for workflow validation".into(),
        );
    }

    repository.write_file(
        planner_requirement_file.to_str().ok_or_else(|| {
            AppError::InvalidPath(format!(
                "Invalid planner requirement path: {}",
                planner_requirement_file.display()
            ))
        })?,
        &serde_yaml::to_string(&planner_requirement_yaml).map_err(|err| {
            AppError::InternalError(format!(
                "Failed to serialize planner requirement YAML: {}",
                err
            ))
        })?,
    )?;

    // Requirement 2: ready for implementer
    let implementer_requirement_id = generate_mock_id();
    let implementer_requirement_file =
        requirements_dir.join(format!("impl-{}.yml", config.mock_tag));

    let mut implementer_requirement_yaml: serde_yaml::Value =
        serde_yaml::from_str(mock_requirement_template).map_err(|e| {
            AppError::InternalError(format!("Failed to parse mock requirement template: {}", e))
        })?;

    if let Some(mapping) = implementer_requirement_yaml.as_mapping_mut() {
        mapping.insert("id".into(), implementer_requirement_id.clone().into());
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
        mapping.insert("implementation_ready".into(), true.into());
        mapping.insert("planner_request_reason".into(), "".into());
    }

    repository.write_file(
        implementer_requirement_file.to_str().ok_or_else(|| {
            AppError::InvalidPath(format!(
                "Invalid implementer requirement path: {}",
                implementer_requirement_file.display()
            ))
        })?,
        &serde_yaml::to_string(&implementer_requirement_yaml).map_err(|err| {
            AppError::InternalError(format!(
                "Failed to serialize implementer requirement YAML: {}",
                err
            ))
        })?,
    )?;

    // Ensure all tag-matched decided events have requirement_id.
    let planner_event_set: HashSet<&str> =
        planner_source_event_ids.iter().map(|event_id| event_id.as_str()).collect();
    for decided_file in &decided_mock_files {
        if let Some(event_id) = mock_event_id_from_path(decided_file, &config.mock_tag) {
            let assigned_requirement_id = if planner_event_set.contains(event_id.as_str()) {
                &planner_requirement_id
            } else {
                &implementer_requirement_id
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

            let content = match repository.read_file(decided_file_str) {
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
                serde_yaml::Value::String("requirement_id".to_string()),
                serde_yaml::Value::String(assigned_requirement_id.to_string()),
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

            if let Err(err) = repository.write_file(decided_file_str, &updated_content) {
                println!(
                    "::warning::Failed to write decided event file {}: {}",
                    decided_file.display(),
                    err
                );
            }
        }
    }

    // Commit and push (include moved/deleted files and decided updates)
    let mut files: Vec<&Path> =
        vec![planner_requirement_file.as_path(), implementer_requirement_file.as_path()];
    for f in &decided_mock_files {
        files.push(f.as_path());
    }
    for f in &moved_src_files {
        files.push(f.as_path());
    }
    service.commit_and_push(
        &format!("[{}] decider: mock requirements", config.mock_tag),
        &files,
        &branch_name,
    )?;

    // Create PR
    let pr = service.create_pr(
        &branch_name,
        &config.jules_worker_branch,
        &format!("[{}] Decider triage", config.mock_tag),
        &format!("Mock decider run for workflow validation.\n\nMock tag: `{}`\n\nCreated requirements:\n- `{}` (requires analysis)\n- `{}` (ready for impl)",
            config.mock_tag, planner_requirement_id, implementer_requirement_id),
    )?;

    println!("Mock decider: created PR #{} ({})", pr.number, pr.url);

    let output = MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    };

    service.finish(&output)?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RepositoryFilesystem;
    use crate::testing::{FakeGit, FakeGitHub, TestStore};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_config() -> MockConfig {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Decider, "jules-decider-".to_string());
        MockConfig {
            mock_tag: "mock-test-decider".to_string(),
            branch_prefixes: prefixes,
            jlo_target_branch: "main".to_string(),
            jules_worker_branch: "jules".to_string(),
            issue_labels: vec!["bugs".to_string()],
        }
    }

    #[test]
    fn mock_decider_processes_events_and_creates_requirements() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        repository
            .write_file(
                ".jules/exchange/events/pending/mock-test-decider-event1.yml",
                "id: event1\nsummary: s1",
            )
            .unwrap();
        repository
            .write_file(
                ".jules/exchange/events/pending/mock-test-decider-event2.yml",
                "id: event2\nsummary: s2",
            )
            .unwrap();

        let result = execute_mock(&jules_path, &config, &git, &github, &repository);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.mock_branch.starts_with("jules-decider-"));
        assert_eq!(output.mock_pr_number, 101);

        let req_dir = ".jules/exchange/requirements";
        let req_files = repository.list_dir(req_dir).unwrap();
        let planner_req = req_files
            .iter()
            .find(|p| p.to_string_lossy().contains("planner-mock-test-decider"))
            .expect("planner req missing");
        let impl_req = req_files
            .iter()
            .find(|p| p.to_string_lossy().contains("impl-mock-test-decider"))
            .expect("implementer req missing");

        assert!(repository.file_exists(&planner_req.to_string_lossy()));
        assert!(repository.file_exists(&impl_req.to_string_lossy()));

        let planner_content = repository.read_file(&planner_req.to_string_lossy()).unwrap();
        let planner_yaml: serde_yaml::Value = serde_yaml::from_str(&planner_content).unwrap();
        assert_eq!(planner_yaml["implementation_ready"].as_bool(), Some(false));
        assert!(!planner_yaml["planner_request_reason"].as_str().unwrap_or("").trim().is_empty());

        let impl_content = repository.read_file(&impl_req.to_string_lossy()).unwrap();
        let impl_yaml: serde_yaml::Value = serde_yaml::from_str(&impl_content).unwrap();
        assert_eq!(impl_yaml["implementation_ready"].as_bool(), Some(true));
        assert_eq!(impl_yaml["planner_request_reason"].as_str(), Some(""));

        assert!(
            !repository.file_exists(".jules/exchange/events/pending/mock-test-decider-event1.yml")
        );
        assert!(
            repository.file_exists(".jules/exchange/events/decided/mock-test-decider-event1.yml")
        );
    }

    #[test]
    fn mock_decider_fails_with_insufficient_events() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        repository
            .write_file(".jules/exchange/events/pending/mock-test-decider-event1.yml", "id: event1")
            .unwrap();

        let result = execute_mock(&jules_path, &config, &git, &github, &repository);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(AppError::Config(ConfigError::Invalid(ref msg))) if msg.contains("requires at least 2 decided events"))
        );
    }
}
