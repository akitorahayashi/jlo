use std::path::Path;

use chrono::Utc;

use super::super::mock::mock_execution::{MOCK_ASSETS, generate_mock_id};
use crate::app::commands::run::input::{detect_repository_source, load_mock_config};
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::repository::paths::jules;
use crate::domain::{
    AppError, Layer, MockConfig, MockOutput, PromptAssetLoader, RoleId, RunConfig, RunOptions,
};
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};

use super::super::role_session::{dispatch_session, print_role_preview, validate_role_exists};
use super::super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct InnovatorsLayer;

impl<W> LayerStrategy<W> for InnovatorsLayer
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
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn Git,
        github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if options.mock {
            let mock_config = load_mock_config(jules_path, options, repository)?;
            let output = execute_mock(jules_path, options, &mock_config, git, github, repository)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::super::mock::mock_execution::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::super::mock::mock_execution::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec![options.role.clone().unwrap_or_else(|| "mock".to_string())],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            options.prompt_preview,
            options.branch.as_deref(),
            options.role.as_deref(),
            options.task.as_deref(),
            config,
            git,
            repository,
            client_factory,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_real<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    role: Option<&str>,
    task: Option<&str>,
    config: &RunConfig,
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
    let role = role
        .ok_or_else(|| AppError::MissingArgument("Role is required for innovators".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Innovators, role_id.as_str(), repository)?;

    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_worker_branch.clone());

    let task = task.ok_or_else(|| {
        AppError::MissingArgument(
            "--task is required for innovators (e.g. create_idea, refine_idea_and_create_proposal, create_proposal)".to_string(),
        )
    })?;
    let task_content = resolve_innovator_task(jules_path, task, repository)?;

    if prompt_preview {
        print_role_preview(jules_path, Layer::Innovators, &role_id, &starting_branch, repository);
        let assembled = assemble_innovator_prompt(
            jules_path,
            role_id.as_str(),
            task,
            &task_content,
            repository,
        )?;
        println!("  Assembled prompt: {} chars", assembled.len());
        println!("\nWould execute 1 session");
        return Ok(RunResult {
            roles: vec![role.to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let source = detect_repository_source(git)?;
    let assembled =
        assemble_innovator_prompt(jules_path, role_id.as_str(), task, &task_content, repository)?;
    let client = client_factory.create()?;

    let session_id = dispatch_session(
        Layer::Innovators,
        &role_id,
        assembled,
        &source,
        starting_branch,
        client.as_ref(),
    )?;

    Ok(RunResult {
        roles: vec![role.to_string()],
        prompt_preview: false,
        sessions: vec![session_id],
        cleanup_requirement: None,
    })
}

fn assemble_innovator_prompt<
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
    role: &str,
    task_name: &str,
    task: &str,
    repository: &W,
) -> Result<String, AppError> {
    let context = PromptContext::new()
        .with_var("role", role)
        .with_var("task_name", task_name)
        .with_var("task", task);

    assemble_prompt(jules_path, Layer::Innovators, &context, repository)
        .map(|p: AssembledPrompt| p.content)
        .map_err(|e| AppError::InternalError(e.to_string()))
}

fn resolve_innovator_task<W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader>(
    jules_path: &Path,
    task: &str,
    repository: &W,
) -> Result<String, AppError> {
    let filename = match task {
        "create_idea" => "create_idea.yml",
        "refine_idea_and_create_proposal" => "refine_idea_and_create_proposal.yml",
        "create_proposal" => "create_proposal.yml",
        _ => {
            return Err(AppError::Validation(format!("Invalid innovator task '{}'", task)));
        }
    };
    let task_path = jules::tasks_dir(jules_path, Layer::Innovators).join(filename);
    repository.read_file(&task_path.to_string_lossy()).map_err(|_| {
        AppError::Validation(format!(
            "No task file for innovators task '{}': expected {}",
            task,
            task_path.display()
        ))
    })
}

// Template placeholder constants (must match src/assets/mock/innovator_idea.yml)
const TMPL_ID: &str = "mock01";
const TMPL_PERSONA: &str = "mock-persona";
const TMPL_DATE: &str = "2026-02-05";
const TMPL_TAG: &str = "test-tag";

fn sanitize_yaml_value(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | ':' | '#' | '\'' | '"' | '{' | '}' | '[' | ']'))
        .collect()
}

fn execute_mock<G, H, W>(
    jules_path: &Path,
    options: &RunOptions,
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
    let role = options.role.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Role (persona) is required for innovators".to_string())
    })?;

    let task = options.task.as_deref().ok_or_else(|| {
        AppError::MissingArgument(
            "--task is required for innovators (create_idea, refine_idea_and_create_proposal, create_proposal)".to_string(),
        )
    })?;

    if task != "create_idea"
        && task != "refine_idea_and_create_proposal"
        && task != "create_proposal"
    {
        return Err(AppError::Validation(format!(
            "Invalid innovator task '{}': expected create_idea, refine_idea_and_create_proposal, or create_proposal",
            task
        )));
    }

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

    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_worker_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    let room_dir_str =
        room_dir.to_str().ok_or_else(|| AppError::Validation("Invalid room path".to_string()))?;
    repository.create_dir_all(room_dir_str)?;

    println!("Mock innovators: task={} for {}", task, role);

    if task == "create_idea" {
        let mock_idea_template = MOCK_ASSETS
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

        repository.write_file(idea_path_str, &idea_content)?;
        let files: Vec<&Path> = vec![idea_path.as_path()];
        git.commit_files(
            &format!("[{}] innovator: mock creation (create idea)", config.mock_tag),
            &files,
        )?;
    } else if task == "refine_idea_and_create_proposal" {
        if repository.file_exists(idea_path_str) {
            repository.remove_file(idea_path_str)?;
            let files: Vec<&Path> = vec![idea_path.as_path()];
            git.commit_files(
                &format!("[{}] innovator: mock refinement (remove idea)", config.mock_tag),
                &files,
            )?;
        }
    } else {
        let proposal_path = room_dir.join("proposal.yml");
        let proposal_path_str = proposal_path
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid proposal.yml path".to_string()))?;
        let proposal_id = generate_mock_id();
        let safe_tag = sanitize_yaml_value(&config.mock_tag);
        let proposal_content = format!(
            "schema_version: 1\nid: \"{}\"\npersona: \"{}\"\ncreated_at: \"{}\"\ntitle: \"Mock direct proposal for {}\"\nproblem: |\n  Mock direct proposal generated without observer pass.\n  Mock tag: {}\nintroduction: |\n  Direct path for innovators entry-point execution.\nimportance: |\n  Validate single-pass proposal publication behavior.\nimpact_surface:\n  - \"workflow\"\nimplementation_cost: \"medium\"\nconsistency_risks:\n  - \"Bypasses observer feedback loop by design for direct mode\"\nverification_signals:\n  - \"Issue is created from proposal.yml in the same run\"\n",
            proposal_id,
            role,
            Utc::now().format("%Y-%m-%d"),
            role,
            safe_tag
        );
        repository.write_file(proposal_path_str, &proposal_content)?;
        let files: Vec<&Path> = vec![proposal_path.as_path()];
        git.commit_files(
            &format!("[{}] innovator: mock direct proposal", config.mock_tag),
            &files,
        )?;
    }

    git.push_branch(&branch_name, false)?;

    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_worker_branch,
        &format!("[{}] Innovator {} {}", config.mock_tag, role, task),
        &format!(
            "Mock innovator run for workflow validation.\n\n\
             Mock tag: `{}`\nPersona: `{}`\nTask: {}",
            config.mock_tag, role, task
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
    use crate::ports::RepositoryFilesystem;
    use crate::testing::{FakeGit, FakeGitHub, TestStore};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_config() -> MockConfig {
        let mut prefixes = HashMap::new();
        prefixes.insert(Layer::Innovators, "jules-innovator-".to_string());
        MockConfig {
            mock_tag: "mock-test-001".to_string(),
            branch_prefixes: prefixes,
            jlo_target_branch: "main".to_string(),
            jules_worker_branch: "jules".to_string(),
            issue_labels: vec![],
        }
    }

    #[test]
    fn mock_innovator_creates_idea_with_create_idea_task() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            task: Some("create_idea".to_string()),
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &repository);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.mock_branch.starts_with("jules-innovator-"));
        assert_eq!(output.mock_pr_number, 101);

        // idea.yml should now exist
        let idea_path = jules_path.join("exchange/innovators/alice/idea.yml");
        assert!(repository.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_removes_idea_with_refine_idea_and_create_proposal_task() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        // Pre-populate idea.yml
        let idea_path = jules_path.join("exchange/innovators/alice/idea.yml");
        repository.write_file(idea_path.to_str().unwrap(), "existing idea").unwrap();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            task: Some("refine_idea_and_create_proposal".to_string()),
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &repository);
        assert!(result.is_ok());

        // idea.yml should be removed
        assert!(!repository.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_create_then_refine_is_deterministic() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let idea_path = jules_path.join("exchange/innovators/alice/idea.yml");

        // create_idea task: creates idea.yml
        let create_options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            task: Some("create_idea".to_string()),
        };
        let _ = execute_mock(&jules_path, &create_options, &config, &git, &github, &repository)
            .unwrap();
        assert!(repository.file_exists(idea_path.to_str().unwrap()));

        // refine_proposal task: removes idea.yml
        let refine_options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            task: Some("refine_idea_and_create_proposal".to_string()),
        };
        let _ = execute_mock(&jules_path, &refine_options, &config, &git, &github, &repository)
            .unwrap();
        assert!(!repository.file_exists(idea_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_direct_task_creates_proposal() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            task: Some("create_proposal".to_string()),
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &repository);
        assert!(result.is_ok());

        let proposal_path = jules_path.join("exchange/innovators/alice/proposal.yml");
        assert!(repository.file_exists(proposal_path.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_rejects_missing_task() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            task: None,
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &repository);
        assert!(result.is_err());
    }

    #[test]
    fn mock_innovator_rejects_invalid_task() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let options = RunOptions {
            layer: Layer::Innovators,
            role: Some("alice".to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: true,
            task: Some("invalid".to_string()),
        };

        let result = execute_mock(&jules_path, &options, &config, &git, &github, &repository);
        assert!(result.is_err());
    }
}
