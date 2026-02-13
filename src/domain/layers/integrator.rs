use std::path::Path;

use crate::domain::configuration::loader::detect_repository_source;
use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::prompt_assembly::{AssembledPrompt, PromptContext, assemble_prompt};
use crate::domain::workspace::paths::jules;
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
    // Validate branch override if provided
    if let Some(b) = branch {
        if !validate_safe_path_component(b) {
            return Err(AppError::Validation(format!(
                "Invalid branch '{}': must be a safe path component",
                b,
            )));
        }
    }

    // Integrator starts from the target branch (same basis as implementer output routing)
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.default_branch.clone());

    // Resolve implementer branch prefix from its contracts for discovery
    let implementer_prefix = load_implementer_branch_prefix(jules_path, workspace)?;

    // Preflight: discover candidate branches before Jules API session creation
    let candidates = discover_candidate_branches(git, &implementer_prefix)?;

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

/// Read the implementer branch prefix from its contracts.yml to drive discovery.
fn load_implementer_branch_prefix<W: WorkspaceStore>(
    jules_path: &Path,
    workspace: &W,
) -> Result<String, AppError> {
    let contracts_path = jules::contracts(jules_path, Layer::Implementer);
    let contracts_path_str = contracts_path.to_string_lossy();

    let content = workspace.read_file(&contracts_path_str).map_err(|_| {
        AppError::Validation(format!(
            "Cannot read implementer contracts at {}: required for branch discovery",
            contracts_path.display()
        ))
    })?;

    let value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| AppError::Validation(format!("Invalid implementer contracts YAML: {}", e)))?;

    value
        .get("branch_prefix")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            AppError::Validation(
                "Implementer contracts.yml missing required 'branch_prefix' field".to_string(),
            )
        })
}

/// Discover remote implementer branches matching the branch prefix policy.
///
/// Fails explicitly if no candidate branches exist.
fn discover_candidate_branches<G: GitPort + ?Sized>(
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
        .filter(|name| {
            name.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
        })
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
