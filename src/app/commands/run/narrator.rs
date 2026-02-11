//! Narrator layer execution (single-role, not issue-driven).
//!
//! The Narrator produces `.jules/exchange/changes.yml` summarizing recent codebase changes.
//! Prompt structure is fully declared in `prompt_assembly.j2`; this module only
//! computes the PromptContext variables the template needs.

use std::path::Path;

use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::narrator_logic::{RangeContext, determine_range_strategy};
use super::prompt::assemble_single_role_prompt_with_context;
use crate::adapters::jules_client_http::HttpJulesClient;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, PromptContext};
use crate::ports::{AutomationMode, GitPort, JulesClient, SessionRequest, WorkspaceStore};

/// Execute the Narrator layer.
pub fn execute<G, W>(
    jules_path: &Path,
    prompt_preview: bool,
    branch: Option<&str>,
    git: &G,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    G: GitPort,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    let config = load_config(jules_path)?;
    let changes_path = exchange_changes_path(jules_path)?;
    let had_previous_changes = workspace.file_exists(&changes_path);

    // Determine starting branch (Narrator always uses jules branch)
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_branch.clone());

    // Determine commit range
    let range = determine_range(&changes_path, git, workspace)?;

    // Check if there are any non-excluded changes in the range
    let pathspec = &[".", ":(exclude).jules"];
    let has_changes = git.has_changes(&range.from_commit, &range.to_commit, pathspec)?;

    if !has_changes {
        println!("No codebase changes detected (excluding .jules/). Skipping Narrator.");
        return Ok(RunResult {
            roles: vec!["narrator".to_string()],
            prompt_preview,
            sessions: vec![],
        });
    }

    let prompt = assemble_narrator_prompt(jules_path, &range, git, workspace)?;

    if prompt_preview {
        println!("=== Prompt Preview: Narrator ===");
        println!("Starting branch: {}\n", starting_branch);
        println!("{}", prompt);
        return Ok(RunResult {
            roles: vec!["narrator".to_string()],
            prompt_preview: true,
            sessions: vec![],
        });
    }

    if had_previous_changes {
        workspace.remove_file(&changes_path)?;
        println!("Removed previous .jules/exchange/changes.yml after reading created_at cursor.");
    }

    // Create session
    let source = detect_repository_source()?;
    let client = HttpJulesClient::from_env_with_config(&config.jules)?;
    let request = SessionRequest {
        prompt,
        source,
        starting_branch,
        require_plan_approval: false,
        automation_mode: AutomationMode::AutoCreatePr,
    };

    match client.create_session(request) {
        Ok(response) => {
            println!("✅ Narrator session created: {}", response.session_id);
            Ok(RunResult {
                roles: vec!["narrator".to_string()],
                prompt_preview: false,
                sessions: vec![response.session_id],
            })
        }
        Err(e) => {
            println!("❌ Failed to create Narrator session: {}", e);
            Err(e)
        }
    }
}

/// Assemble the narrator prompt via .j2 template with PromptContext variables.
fn assemble_narrator_prompt<G: GitPort, W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    range: &RangeContext,
    git: &G,
    workspace: &W,
) -> Result<String, AppError> {
    let run_mode = match range.selection_mode.as_str() {
        "incremental" => "overwrite",
        other => other,
    };

    let mut prompt_context = PromptContext::new()
        .with_var("run_mode", run_mode)
        .with_var("range_description", build_range_description(range));

    // For overwrite: provide the commit list since cursor so narrator has them in-context
    if run_mode == "overwrite" {
        let commits_text = fetch_commits_since_cursor(git, range)?;
        prompt_context = prompt_context.with_var("commits_since_cursor", commits_text);
    }

    assemble_single_role_prompt_with_context(
        jules_path,
        Layer::Narrators,
        &prompt_context,
        workspace,
    )
}

/// Fetch commit SHA + message lines since the cursor timestamp.
fn fetch_commits_since_cursor<G: GitPort>(
    git: &G,
    range: &RangeContext,
) -> Result<String, AppError> {
    let since = range.changes_since.as_deref().unwrap_or("");
    if since.is_empty() {
        return Ok(String::new());
    }
    let after_arg = format!("--after={}", since);
    let output = git.run_command(
        &[
            "log",
            "--oneline",
            &after_arg,
            "--format=%H %ai %s",
            "--",
            ".",
            ":(exclude).jules",
            ":(exclude).jlo",
        ],
        None,
    )?;
    Ok(output.trim().to_string())
}

/// Build a human-readable description of the commit range for the model.
fn build_range_description(range: &RangeContext) -> String {
    let short_from = &range.from_commit[..7.min(range.from_commit.len())];
    let short_to = &range.to_commit[..7.min(range.to_commit.len())];

    match range.selection_mode.as_str() {
        "incremental" => {
            let since = range.changes_since.as_deref().unwrap_or("unknown");
            format!("Summarize changes since {} (commits {}..{}).", since, short_from, short_to)
        }
        "bootstrap" => {
            let detail = if range.selection_detail.is_empty() {
                "recent commits".to_string()
            } else {
                range.selection_detail.clone()
            };
            format!("First summary — {} (commits {}..{}).", detail, short_from, short_to)
        }
        _ => {
            format!("Commits {}..{}.", short_from, short_to)
        }
    }
}

/// Determine the commit range for the summary.
fn determine_range<G, W>(
    changes_path: &str,
    git: &G,
    workspace: &W,
) -> Result<RangeContext, AppError>
where
    G: GitPort,
    W: WorkspaceStore,
{
    let head_sha = git.get_head_sha()?;
    let changes_content = if workspace.file_exists(changes_path) {
        Some(workspace.read_file(changes_path)?)
    } else {
        None
    };

    determine_range_strategy(
        &head_sha,
        changes_content.as_deref(),
        |sha, n| git.get_nth_ancestor(sha, n),
        |sha, timestamp| {
            let commit = get_commit_before_timestamp(git, sha, timestamp)?;
            if let Some(ref base) = commit
                && !git.commit_exists(base)
            {
                return Err(AppError::Validation(format!(
                    "Resolved base commit does not exist: {}",
                    base
                )));
            }
            Ok(commit)
        },
    )
}

fn get_commit_before_timestamp<G: GitPort>(
    git: &G,
    head_sha: &str,
    timestamp: &str,
) -> Result<Option<String>, AppError> {
    let before_arg = format!("--before={}", timestamp);
    let output = git.run_command(&["rev-list", "-1", &before_arg, head_sha], None)?;
    let commit = output.trim();
    if commit.is_empty() { Ok(None) } else { Ok(Some(commit.to_string())) }
}

fn exchange_changes_path(jules_path: &Path) -> Result<String, AppError> {
    jules::exchange_changes(jules_path)
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Validation("Jules path contains invalid unicode".to_string()))
}
