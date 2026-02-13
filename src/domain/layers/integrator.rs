use std::path::Path;

use crate::domain::configuration::loader::detect_repository_source;
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::{AppError, Layer, RunConfig, RunOptions};
use crate::ports::{AutomationMode, GitHubPort, GitPort, SessionRequest, WorkspaceStore};

use super::strategy::{JulesClientFactory, LayerStrategy, RunResult};

pub struct IntegratorLayer;

impl<W> LayerStrategy<W> for IntegratorLayer
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    fn execute(
        &self,
        jules_path: &Path,
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn GitPort,
        _github: &dyn GitHubPort,
        workspace: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError> {
        if options.mock {
            return Err(AppError::Validation("Integrator does not support mock mode".to_string()));
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
    // Integrator starts from the implementer target branch (default_branch)
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.default_branch.clone());

    // Preflight: discover candidate branches before Jules API session creation
    let candidates = discover_candidate_branches(git)?;

    if prompt_preview {
        println!("=== Prompt Preview: Integrator ===");
        println!("Starting branch: {}", starting_branch);
        println!("Candidate branches ({}):", candidates.len());
        for branch_name in &candidates {
            println!("  - {}", branch_name);
        }
        println!();

        let prompt =
            assemble_integrator_prompt(jules_path, &starting_branch, &candidates, git, workspace)?;
        println!("{}", prompt);

        return Ok(RunResult {
            roles: vec!["integrator".to_string()],
            prompt_preview: true,
            sessions: vec![],
            cleanup_requirement: None,
        });
    }

    let source = detect_repository_source(git)?;
    let client = client_factory.create()?;

    let prompt =
        assemble_integrator_prompt(jules_path, &starting_branch, &candidates, git, workspace)?;

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

/// Discover remote implementer branches matching the branch prefix policy.
///
/// Fails explicitly if no candidate branches exist.
fn discover_candidate_branches<G: GitPort + ?Sized>(git: &G) -> Result<Vec<String>, AppError> {
    // Fetch latest remote state
    git.fetch("origin")?;

    // List remote branches matching implementer prefix
    let output = git.run_command(
        &["branch", "-r", "--list", "origin/jules-implementer-*", "--format=%(refname:short)"],
        None,
    )?;

    let candidates: Vec<String> = output
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.strip_prefix("origin/").unwrap_or(line).to_string())
        .collect();

    if candidates.is_empty() {
        return Err(AppError::Validation(
            "No remote jules-implementer-* branches found. Nothing to integrate.".to_string(),
        ));
    }

    println!(
        "Preflight: discovered {} candidate branch(es): {}",
        candidates.len(),
        candidates.join(", ")
    );

    Ok(candidates)
}

fn assemble_integrator_prompt<
    G: GitPort + ?Sized,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
>(
    jules_path: &Path,
    starting_branch: &str,
    candidates: &[String],
    git: &G,
    workspace: &W,
) -> Result<String, AppError> {
    let source = detect_repository_source(git)?;

    let candidate_list =
        candidates.iter().map(|b| format!("- {}", b)).collect::<Vec<_>>().join("\n");

    let context = PromptContext::new()
        .with_var("target_branch", starting_branch)
        .with_var("candidate_branches", candidate_list)
        .with_var("repository", source);

    assemble_prompt(jules_path, Layer::Integrator, &context, workspace)
        .map(|p: AssembledPrompt| p.content)
        .map_err(|e| AppError::InternalError(e.to_string()))
}
