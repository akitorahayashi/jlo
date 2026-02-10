//! Narrator layer execution (single-role, not issue-driven).
//!
//! The Narrator produces `.jules/changes/latest.yml` summarizing recent codebase changes.

use std::path::Path;

use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::narrator_logic::{
    GitContext, MAX_COMMITS, NarratorGitData, RangeContext, analyze_git_context,
    determine_range_strategy,
};
use super::prompt::assemble_single_role_prompt_with_context;
use crate::adapters::jules_client_http::HttpJulesClient;
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
    let latest_path = latest_summary_path(jules_path)?;
    let had_previous_latest = workspace.file_exists(&latest_path);

    // Determine starting branch (Narrator always uses jules branch)
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_branch.clone());

    // Perform git preflight to determine range and check for changes
    let git_context = match collect_git_context(jules_path, git, workspace)? {
        Some(ctx) => ctx,
        None => {
            // No changes - exit successfully without creating a session
            println!("No codebase changes detected (excluding .jules/). Skipping Narrator.");
            return Ok(RunResult {
                roles: vec!["narrator".to_string()],
                prompt_preview,
                sessions: vec![],
            });
        }
    };

    if prompt_preview {
        execute_prompt_preview(jules_path, &starting_branch, &git_context, workspace)?;
        return Ok(RunResult {
            roles: vec!["narrator".to_string()],
            prompt_preview: true,
            sessions: vec![],
        });
    }

    if had_previous_latest {
        workspace.remove_file(&latest_path)?;
        println!("Removed previous .jules/changes/latest.yml after reading created_at cursor.");
    }

    // Create session with git context injected into prompt
    let source = detect_repository_source()?;
    let prompt = build_narrator_prompt(jules_path, &git_context, workspace)?;

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

/// Collect git context for the Narrator, returning None if no changes.
fn collect_git_context<G, W>(
    jules_path: &Path,
    git: &G,
    workspace: &W,
) -> Result<Option<GitContext>, AppError>
where
    G: GitPort,
    W: WorkspaceStore + Clone + Send + Sync + 'static,
{
    // Construct path to latest.yml relative to workspace root or absolute
    let latest_path_str = latest_summary_path(jules_path)?;

    let range = determine_range(&latest_path_str, git, workspace)?;

    // Check if there are any non-excluded changes in the range
    let pathspec = &[".", ":(exclude).jules"];
    let has_changes = git.has_changes(&range.from_commit, &range.to_commit, pathspec)?;

    let (commits_total, commits, diffstat) = if has_changes {
        (
            git.count_commits(&range.from_commit, &range.to_commit, pathspec)?,
            git.collect_commits(&range.from_commit, &range.to_commit, pathspec, MAX_COMMITS)?,
            git.get_diffstat(&range.from_commit, &range.to_commit, pathspec)?,
        )
    } else {
        (0, Vec::new(), crate::ports::DiffStat::default())
    };

    let data = NarratorGitData { range, has_changes, commits_total, commits, diffstat };

    Ok(analyze_git_context(data))
}

/// Determine the commit range for the summary.
fn determine_range<G, W>(
    latest_path: &str,
    git: &G,
    workspace: &W,
) -> Result<RangeContext, AppError>
where
    G: GitPort,
    W: WorkspaceStore,
{
    let head_sha = git.get_head_sha()?;
    let latest_content = if workspace.file_exists(latest_path) {
        Some(workspace.read_file(latest_path)?)
    } else {
        None
    };

    determine_range_strategy(
        &head_sha,
        latest_content.as_deref(),
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

/// Build the full Narrator prompt with git context injected.
fn build_narrator_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    ctx: &GitContext,
    workspace: &W,
) -> Result<String, AppError> {
    let base_prompt = assemble_narrator_base_prompt(jules_path, ctx, workspace)?;

    // Build the git context section
    let mut context_section = String::new();
    context_section.push_str("\n---\n# Runner-Provided Git Context\n\n");

    context_section.push_str("## Range\n");
    context_section.push_str(&format!("- from_commit: {}\n", ctx.range.from_commit));
    context_section.push_str(&format!("- to_commit: {}\n", ctx.range.to_commit));
    context_section.push_str(&format!("- selection_mode: {}\n", ctx.range.selection_mode));
    if let Some(changes_since) = ctx.range.changes_since.as_deref() {
        context_section.push_str(&format!("- changes_since: {}\n", changes_since));
    }
    if !ctx.range.selection_detail.is_empty() {
        context_section.push_str(&format!("- selection_detail: {}\n", ctx.range.selection_detail));
    }

    context_section.push_str("\n## Stats\n");
    context_section.push_str(&format!("- commits_total: {}\n", ctx.stats.commits_total));
    context_section.push_str(&format!("- commits_included: {}\n", ctx.stats.commits_included));
    context_section.push_str(&format!("- files_changed: {}\n", ctx.stats.files_changed));
    context_section.push_str(&format!("- insertions: {}\n", ctx.stats.insertions));
    context_section.push_str(&format!("- deletions: {}\n", ctx.stats.deletions));

    context_section.push_str(&format!(
        "\n## Commits ({} of {} total)\n",
        ctx.stats.commits_included, ctx.stats.commits_total
    ));
    for commit in &ctx.commits {
        context_section.push_str(&format!(
            "- {} {}\n",
            &commit.sha[..7.min(commit.sha.len())],
            commit.subject
        ));
    }

    if !ctx.truncation_note.is_empty() {
        context_section.push_str(&format!("\n## Truncation\n{}\n", ctx.truncation_note));
    }

    Ok(format!("{}{}", base_prompt, context_section))
}

/// Execute a prompt preview, showing the assembled prompt and context.
fn execute_prompt_preview<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    starting_branch: &str,
    ctx: &GitContext,
    workspace: &W,
) -> Result<(), AppError> {
    println!("=== Prompt Preview: Narrator ===");
    println!("Starting branch: {}\n", starting_branch);

    println!("--- Git Context ---");
    println!(
        "Range: {}..{} ({})",
        ctx.range.from_commit, ctx.range.to_commit, ctx.range.selection_mode
    );
    if let Some(changes_since) = ctx.range.changes_since.as_deref() {
        println!("Changes since: {}", changes_since);
    }
    println!(
        "Stats: {} commits ({} included), {} files, +{} -{}",
        ctx.stats.commits_total,
        ctx.stats.commits_included,
        ctx.stats.files_changed,
        ctx.stats.insertions,
        ctx.stats.deletions
    );
    if !ctx.truncation_note.is_empty() {
        println!("Truncation: {}", ctx.truncation_note);
    }

    println!("\n--- Prompt (base) ---");
    let prompt = assemble_narrator_base_prompt(jules_path, ctx, workspace)?;
    println!("{}", prompt);

    Ok(())
}

fn assemble_narrator_base_prompt<W: WorkspaceStore + Clone + Send + Sync + 'static>(
    jules_path: &Path,
    ctx: &GitContext,
    workspace: &W,
) -> Result<String, AppError> {
    let prompt_context = PromptContext::new()
        .with_var("run_mode", ctx.range.selection_mode.clone())
        .with_var("changes_since", ctx.range.changes_since.clone().unwrap_or_default());
    assemble_single_role_prompt_with_context(
        jules_path,
        Layer::Narrators,
        &prompt_context,
        workspace,
    )
}

fn latest_summary_path(jules_path: &Path) -> Result<String, AppError> {
    jules_path
        .join("changes/latest.yml")
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Validation("Jules path contains invalid unicode".to_string()))
}
