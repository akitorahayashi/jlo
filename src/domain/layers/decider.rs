use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::domain::configuration::loader::detect_repository_source;
use crate::domain::configuration::mock_loader::load_mock_config;
use crate::domain::layers::mock_utils::{MOCK_ASSETS, generate_mock_id};
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, MockConfig, MockOutput, RunConfig, RunOptions};
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
            workspace,
            client_factory,
        )
    }
}

fn execute_real<W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    config: &RunConfig,
    workspace: &W,
    client_factory: &dyn JulesClientFactory,
) -> Result<RunResult, AppError>
where
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

    let source = detect_repository_source()?;
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

    // Ensure directories exist
    // Using workspace for directory creation if possible, but pending_dir is usually created by observers
    // Use fs for now as in original mock implementation
    std::fs::create_dir_all(&decided_dir)?;
    std::fs::create_dir_all(&requirements_dir)?;

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
    if pending_dir.exists() {
        for entry in std::fs::read_dir(&pending_dir)? {
            let entry = entry?;
            let path = entry.path();
            if mock_event_id_from_path(&path, &config.mock_tag).is_some() {
                let dest = decided_dir.join(path.file_name().ok_or_else(|| {
                    AppError::Validation(format!(
                        "Pending event missing filename: {}",
                        path.display()
                    ))
                })?);
                std::fs::rename(&path, &dest)?;
                moved_src_files.push(path);
            }
        }
    }

    let decided_mock_files = list_mock_decided_files(&decided_dir, &config.mock_tag)?;
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

    let planner_event_id = source_event_ids[0].clone();
    let impl_event_id = source_event_ids[1].clone();

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
            seq.push(planner_event_id.clone().into());
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
            seq.push(impl_event_id.clone().into());
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
    // Extra events (e.g., workflow re-run with same mock tag) are attached to implementer issue.
    for decided_file in &decided_mock_files {
        if let Some(event_id) = mock_event_id_from_path(decided_file, &config.mock_tag) {
            let assigned_issue_id =
                if event_id == planner_event_id { &planner_issue_id } else { &impl_issue_id };

            let content = match std::fs::read_to_string(decided_file) {
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

            if let Err(err) = std::fs::write(decided_file, updated_content) {
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

fn list_mock_decided_files(decided_dir: &Path, mock_tag: &str) -> Result<Vec<PathBuf>, AppError> {
    if !decided_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(decided_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| mock_event_id_from_path(path, mock_tag).is_some())
        .collect();

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{list_mock_decided_files, mock_event_id_from_path};
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

        let files = list_mock_decided_files(&decided_dir, "mock-run-123").expect("list");

        assert_eq!(files.len(), 2);
        assert!(files[0].to_string_lossy().ends_with("mock-mock-run-123-aaaaaa.yml"));
        assert!(files[1].to_string_lossy().ends_with("mock-mock-run-123-bbbbbb.yml"));
    }
}
