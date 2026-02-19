use std::path::Path;

use chrono::Utc;

use super::super::mock::mock_execution::{MOCK_ASSETS, generate_mock_id};
use crate::app::commands::run::RunRuntimeOptions;
use crate::app::commands::run::input::{detect_repository_source, load_mock_config};
use crate::domain::layers::execute::starting_branch::resolve_starting_branch;
use crate::domain::prompt_assemble::{PromptAssetLoader, PromptContext, assemble_prompt};
use crate::domain::{
    AppError, ControlPlaneConfig, Layer, MockConfig, MockOutput, RoleId, RunOptions,
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
        target: &RunOptions,
        runtime: &RunRuntimeOptions,
        config: &ControlPlaneConfig,
        git: &dyn Git,
        github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if runtime.mock {
            let role_str = target.role.as_deref().ok_or_else(|| {
                AppError::MissingArgument("Role is required for innovators".to_string())
            })?;
            let role = RoleId::new(role_str)?;
            let task = target.task.as_deref().ok_or_else(|| {
                AppError::MissingArgument(
                    "--task is required for innovators (create_three_proposals)".to_string(),
                )
            })?;

            let mock_config = load_mock_config(jules_path, repository)?;
            let output =
                execute_mock(jules_path, &role, task, &mock_config, git, github, repository)?;
            // Write mock output
            if std::env::var("GITHUB_OUTPUT").is_ok() {
                super::super::mock::mock_execution::write_github_output(&output).map_err(|e| {
                    AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
                })?;
            } else {
                super::super::mock::mock_execution::print_local(&output);
            }
            return Ok(RunResult {
                roles: vec![target.role.clone().unwrap_or_else(|| "mock".to_string())],
                prompt_preview: false,
                sessions: vec![],
                cleanup_requirement: None,
            });
        }

        execute_real(
            jules_path,
            runtime.prompt_preview,
            runtime.branch.as_deref(),
            target.role.as_deref(),
            target.task.as_deref(),
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
    let role = role
        .ok_or_else(|| AppError::MissingArgument("Role is required for innovators".to_string()))?;

    let role_id = RoleId::new(role)?;
    validate_role_exists(jules_path, Layer::Innovators, role_id.as_str(), repository)?;

    let starting_branch = resolve_starting_branch(Layer::Innovators, config, branch);

    let task = task.ok_or_else(|| {
        AppError::MissingArgument(
            "--task is required for innovators (expected: create_three_proposals)".to_string(),
        )
    })?;
    let task_content = resolve_innovator_task(task)?;

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

    let (prompt, seed_ops) = assemble_prompt(
        jules_path,
        Layer::Innovators,
        &context,
        repository,
        crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset,
    )
    .map_err(|e| AppError::InternalError(e.to_string()))?;
    super::execute_seed_ops(seed_ops, repository)?;
    Ok(prompt.content)
}

fn resolve_innovator_task(task: &str) -> Result<String, AppError> {
    let filename = match task {
        "create_three_proposals" => "create_three_proposals.yml",
        _ => {
            return Err(AppError::Validation(format!("Invalid innovator task '{}'", task)));
        }
    };
    let catalog_path = format!("innovators/tasks/{}", filename);
    crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset(&catalog_path)
        .ok_or_else(|| {
            AppError::Validation(format!(
                "No task file for innovators task '{}': expected prompt-assemble://{}",
                task, catalog_path
            ))
        })
}

fn sanitize_yaml_value(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | ':' | '#' | '\'' | '"' | '{' | '}' | '[' | ']'))
        .collect()
}

fn load_mock_asset_text(asset_name: &str) -> Result<&'static str, AppError> {
    MOCK_ASSETS
        .get_file(asset_name)
        .ok_or_else(|| AppError::InternalError(format!("Mock asset missing: {}", asset_name)))?
        .contents_utf8()
        .ok_or_else(|| AppError::InternalError(format!("Invalid UTF-8 in {}", asset_name)))
}

fn execute_mock<G, H, W>(
    jules_path: &Path,
    role: &RoleId,
    task: &str,
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
    if task != "create_three_proposals" {
        return Err(AppError::Validation(format!(
            "Invalid innovator task '{}': expected create_three_proposals",
            task
        )));
    }

    let proposals_dir = crate::domain::exchange::proposals::paths::proposals_dir(jules_path);
    let proposals_dir_str = proposals_dir
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid proposals path".to_string()))?;
    repository.create_dir_all(proposals_dir_str)?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Innovators, &timestamp)?;

    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_worker_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    println!("Mock innovators: task={} for {}", task, role);

    let safe_tag = sanitize_yaml_value(&config.mock_tag);
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut created_paths = Vec::new();
    let proposal_template = load_mock_asset_text("innovator_proposal.yml")?;

    for index in 1..=3 {
        let slug = format!("mock-proposal-{}", index);
        let proposal_path = crate::domain::exchange::proposals::paths::proposal_file(
            jules_path,
            role.as_str(),
            &slug,
        );
        let proposal_path_str = proposal_path
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid proposal path".to_string()))?;
        let proposal_title = format!("Mock proposal {} for {}", index, role);
        let proposal_content = proposal_template
            .replace("__ID__", &generate_mock_id())
            .replace("__ROLE__", role.as_str())
            .replace("__DATE__", &today)
            .replace("__TITLE__", &proposal_title)
            .replace("__INDEX__", &index.to_string())
            .replace("__TAG__", &safe_tag);
        repository.write_file(proposal_path_str, &proposal_content)?;
        created_paths.push(proposal_path);
    }

    let files: Vec<&Path> = created_paths.iter().map(|path| path.as_path()).collect();
    git.commit_files(&format!("[{}] innovator: mock three proposals", config.mock_tag), &files)?;

    git.push_branch(&branch_name, false)?;

    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_worker_branch,
        &format!("[{}] Innovator {} {}", config.mock_tag, role, task),
        &format!(
            "Mock innovator run for workflow validation.\n\n\
             Mock tag: `{}`\nRole: `{}`\nTask: {}",
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
    fn mock_innovator_creates_three_proposals() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let role_id = RoleId::new("alice").unwrap();
        let task = "create_three_proposals";

        let result = execute_mock(&jules_path, &role_id, task, &config, &git, &github, &repository);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.mock_branch.starts_with("jules-innovator-"));
        assert_eq!(output.mock_pr_number, 101);

        let p1 = jules_path.join("exchange/proposals/alice-mock-proposal-1.yml");
        let p2 = jules_path.join("exchange/proposals/alice-mock-proposal-2.yml");
        let p3 = jules_path.join("exchange/proposals/alice-mock-proposal-3.yml");
        assert!(repository.file_exists(p1.to_str().unwrap()));
        assert!(repository.file_exists(p2.to_str().unwrap()));
        assert!(repository.file_exists(p3.to_str().unwrap()));
    }

    #[test]
    fn mock_innovator_rejects_invalid_task() {
        // This test was checking missing task in options, but execute_mock now assumes task is present (passed as arg).
        // The check moved to execute(). We can test that RoleId validation happens.
        // But since we pass RoleId, it is already valid.
        // So we can check if task is invalid.

        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let role_id = RoleId::new("alice").unwrap();
        let task = "invalid_task";

        let result = execute_mock(&jules_path, &role_id, task, &config, &git, &github, &repository);
        assert!(result.is_err());
    }

    #[test]
    fn mock_innovator_normalizes_underscored_role_in_proposal_filenames() {
        let jules_path = PathBuf::from(".jules");
        let repository = TestStore::new().with_exists(true);
        let git = FakeGit::new();
        let github = FakeGitHub::new();
        let config = make_config();

        let role_id = RoleId::new("leverage_architect").unwrap();
        let task = "create_three_proposals";

        let result = execute_mock(&jules_path, &role_id, task, &config, &git, &github, &repository);
        assert!(result.is_ok());

        let p1 = jules_path.join("exchange/proposals/leverage-architect-mock-proposal-1.yml");
        let p2 = jules_path.join("exchange/proposals/leverage-architect-mock-proposal-2.yml");
        let p3 = jules_path.join("exchange/proposals/leverage-architect-mock-proposal-3.yml");
        assert!(repository.file_exists(p1.to_str().unwrap()));
        assert!(repository.file_exists(p2.to_str().unwrap()));
        assert!(repository.file_exists(p3.to_str().unwrap()));
    }
}
