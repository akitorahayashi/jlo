use std::path::Path;

use serde::Deserialize;

use crate::app::commands::run::RunRuntimeOptions;
use crate::app::commands::run::input::detect_repository_source;
use crate::domain::layers::execute::starting_branch::resolve_starting_branch;
use crate::domain::prompt_assemble::{PromptAssetLoader, PromptContext, assemble_prompt};
use crate::domain::validation::validate_identifier;
use crate::domain::{AppError, ControlPlaneConfig, Layer, RunOptions};
use crate::ports::{
    AutomationMode, Git, GitHub, JloStore, JulesStore, RepositoryFilesystem, SessionRequest,
};

use super::super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

#[derive(Deserialize)]
struct ContractFile {
    branch_prefix: String,
}

pub struct IntegratorLayer;

impl<W> LayerStrategy<W> for IntegratorLayer
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
        _github: &dyn GitHub,
        repository: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if runtime.mock {
            return Err(AppError::Validation("Integrator does not support mock mode".to_string()));
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
    // Validate branch override if provided
    if let Some(b) = branch
        && !validate_identifier(b, false)
    {
        return Err(AppError::Validation(format!(
            "Invalid branch '{}': must be a safe path component",
            b,
        )));
    }

    let starting_branch = resolve_starting_branch(Layer::Integrator, config, branch);

    // Resolve implementer branch prefix from its contracts for discovery
    let implementer_prefix = load_implementer_branch_prefix()?;

    // Preflight: discover candidate branches before Jules API session creation
    let candidates = discover_candidate_branches(git, &implementer_prefix)?;

    let source = detect_repository_source(git)?;

    if prompt_preview {
        println!("=== Prompt Preview: Integrator ===");
        println!("Starting branch: {}", starting_branch);
        println!("Candidate branches ({}):", candidates.len());
        for branch_name in &candidates {
            println!("  - {}", branch_name);
        }
        println!();

        let prompt = assemble_integrator_prompt(
            jules_path,
            &starting_branch,
            &candidates,
            &source,
            repository,
        )?;
        println!("{}", prompt);

        return Ok(RunResult {
            roles: vec!["integrator".to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let client = client_factory.create()?;

    let prompt =
        assemble_integrator_prompt(jules_path, &starting_branch, &candidates, &source, repository)?;

    let request = SessionRequest {
        prompt,
        source,
        starting_branch: starting_branch.clone(),
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    println!("Executing: integrator ({} candidate branches)...", candidates.len());
    let response = client.create_session(request)?;
    println!("  ✅ Session created: {}", response.session_id);

    Ok(RunResult {
        roles: vec!["integrator".to_string()],
        prompt_preview: false,
        sessions: vec![response.session_id],
        cleanup_requirement: None,
    })
}

/// Read the implementer branch prefix from its contracts.yml to drive discovery.
fn load_implementer_branch_prefix() -> Result<String, AppError> {
    let content = crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset(
        "implementer/contracts.yml",
    )
    .ok_or_else(|| {
        AppError::Validation(
            "Cannot read implementer contracts from embedded catalog: required for branch discovery"
                .to_string(),
        )
    })?;

    let contract: ContractFile = serde_yaml::from_str(&content)
        .map_err(|e| AppError::Validation(format!("Invalid implementer contracts YAML: {}", e)))?;

    if contract.branch_prefix.trim().is_empty() {
        return Err(AppError::Validation(
            "Implementer contracts.yml has an empty 'branch_prefix' field".to_string(),
        ));
    }

    Ok(contract.branch_prefix)
}

/// Discover remote implementer branches matching the branch prefix policy.
///
/// Fails explicitly if no candidate branches exist.
fn discover_candidate_branches<G: Git + ?Sized>(
    git: &G,
    implementer_prefix: &str,
) -> Result<Vec<String>, AppError> {
    git.fetch("origin")?;

    let pattern = format!("origin/{}*", implementer_prefix);
    let output =
        git.run_command(&["branch", "-r", "--list", &pattern, "--format=%(refname:short)"], None)?;

    let candidates: Vec<String> = output
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.strip_prefix("origin/").unwrap_or(line).to_string())
        .filter(|name| validate_identifier(name, false))
        .collect();

    if candidates.is_empty() {
        return Err(AppError::Validation(format!(
            "No remote {}* branches found. Nothing to integrate.",
            implementer_prefix
        )));
    }

    println!(
        "Preflight: discovered {} candidate branch(es): {}",
        candidates.len(),
        candidates.join(", ")
    );

    Ok(candidates)
}

fn assemble_integrator_prompt<
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
    starting_branch: &str,
    candidates: &[String],
    source: &str,
    repository: &W,
) -> Result<String, AppError> {
    let candidate_list =
        candidates.iter().map(|b| format!("- {}", b)).collect::<Vec<_>>().join("\n");

    let context = PromptContext::new()
        .with_var("target_branch", starting_branch)
        .with_var("candidate_branches", candidate_list)
        .with_var("repository", source);

    let (prompt, seed_ops) = assemble_prompt(
        jules_path,
        Layer::Integrator,
        &context,
        repository,
        crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset,
    )
    .map_err(|e| AppError::InternalError(e.to_string()))?;
    super::execute_seed_ops(seed_ops, repository)?;
    Ok(prompt.content)
}
