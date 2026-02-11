use std::path::Path;

use crate::adapters::jules_client_http::HttpJulesClient;
use crate::domain::{AppError, Layer};
use crate::ports::{AutomationMode, JulesClient, SessionRequest, WorkspaceStore};

use super::RunOptions;
use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::prompt::assemble_single_role_prompt;

/// Execute the decider layer (single-role, no `--role` required).
pub(crate) fn execute<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    options: &RunOptions,
    workspace: &W,
) -> Result<RunResult, AppError> {
    let config = load_config(jules_path)?;
    let starting_branch = options.branch.clone().unwrap_or_else(|| config.run.jules_branch.clone());

    if options.prompt_preview {
        println!("=== Prompt Preview: Decider ===");
        println!("Starting branch: {}\n", starting_branch);

        let prompt = assemble_single_role_prompt(jules_path, Layer::Decider, workspace)?;
        println!("  Assembled prompt: {} chars", prompt.len());

        println!("\nWould dispatch workflow");
        return Ok(RunResult {
            roles: vec!["decider".to_string()],
            prompt_preview: true,
            sessions: vec![],
        });
    }

    let source = detect_repository_source()?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let prompt = assemble_single_role_prompt(jules_path, Layer::Decider, workspace)?;

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
    })
}
