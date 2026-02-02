//! Narrator layer execution (single-role, not issue-driven).
//!
//! The Narrator produces `.jules/changes/latest.yml` summarizing recent codebase changes.

use std::fs;
use std::path::Path;
use std::process::Command;

use super::RunResult;
use super::config::{detect_repository_source, load_config};
use super::prompt::assemble_single_role_prompt;
use crate::domain::{AppError, Layer};
use crate::ports::{AutomationMode, JulesClient, SessionRequest};
use crate::services::jules_client_http::HttpJulesClient;

/// Maximum number of commits to include in the summary.
const MAX_COMMITS: usize = 50;
/// Maximum number of changed paths to include.
const MAX_CHANGED_PATHS: usize = 100;
/// Number of commits to use for bootstrap when no prior summary exists.
const BOOTSTRAP_COMMIT_COUNT: usize = 20;

/// Range selection context for Narrator.
#[derive(Debug)]
pub struct RangeContext {
    /// The from_commit (exclusive).
    pub from_commit: String,
    /// The to_commit (inclusive, HEAD).
    pub to_commit: String,
    /// Selection mode: "incremental" or "bootstrap".
    pub selection_mode: String,
    /// Selection detail (non-empty when bootstrapping).
    pub selection_detail: String,
}

/// Git context collected for the Narrator prompt.
#[derive(Debug)]
pub struct GitContext {
    pub range: RangeContext,
    pub diffstat: DiffStat,
    pub commits: Vec<CommitInfo>,
    pub changed_paths: Vec<String>,
    pub truncation_note: String,
}

#[derive(Debug, Default)]
pub struct DiffStat {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug)]
pub struct CommitInfo {
    pub sha: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// Execute the Narrator layer.
pub fn execute(
    jules_path: &Path,
    dry_run: bool,
    branch: Option<&str>,
    is_ci: bool,
) -> Result<RunResult, AppError> {
    let config = load_config(jules_path)?;

    // Determine starting branch (Narrator always uses jules branch)
    let starting_branch =
        branch.map(String::from).unwrap_or_else(|| config.run.jules_branch.clone());

    // Perform git preflight to determine range and check for changes
    let git_context = match collect_git_context(jules_path)? {
        Some(ctx) => ctx,
        None => {
            // No changes - exit successfully without creating a session
            println!("No codebase changes detected (excluding .jules/). Skipping Narrator.");
            return Ok(RunResult {
                roles: vec!["narrator".to_string()],
                dry_run,
                sessions: vec![],
            });
        }
    };

    if dry_run {
        execute_dry_run(jules_path, &starting_branch, &git_context)?;
        return Ok(RunResult {
            roles: vec!["narrator".to_string()],
            dry_run: true,
            sessions: vec![],
        });
    }

    // Outside CI, show what would happen but don't create session
    if !is_ci {
        println!(
            "Narrator execution detected {} commits with {} files changed.",
            git_context.commits.len(),
            git_context.diffstat.files_changed
        );
        println!("Run in CI (GITHUB_ACTIONS=true) to create a Jules session.");
        return Ok(RunResult {
            roles: vec!["narrator".to_string()],
            dry_run: false,
            sessions: vec![],
        });
    }

    // CI Execution: Create session with git context injected into prompt
    let source = detect_repository_source()?;
    let prompt = build_narrator_prompt(jules_path, &git_context)?;

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
                dry_run: false,
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
fn collect_git_context(jules_path: &Path) -> Result<Option<GitContext>, AppError> {
    let latest_path =
        jules_path.parent().unwrap_or(Path::new(".")).join(".jules/changes/latest.yml");

    let range = determine_range(&latest_path)?;

    // Check if there are any non-excluded changes in the range
    let has_changes = check_for_changes(&range)?;
    if !has_changes {
        return Ok(None);
    }

    // Collect commits
    let (commits, commits_truncated) = collect_commits(&range)?;

    // Collect changed paths
    let (changed_paths, paths_truncated) = collect_changed_paths(&range)?;

    // Collect diffstat
    let diffstat = collect_diffstat(&range)?;

    // Build truncation note
    let truncation_note = build_truncation_note(commits_truncated, paths_truncated);

    Ok(Some(GitContext { range, diffstat, commits, changed_paths, truncation_note }))
}

/// Determine the commit range for the summary.
fn determine_range(latest_path: &Path) -> Result<RangeContext, AppError> {
    let head_sha = get_head_sha()?;

    if latest_path.exists()
        && let Ok(content) = fs::read_to_string(latest_path)
        && let Some(prev_to_commit) = extract_to_commit(&content)
    {
        // Verify the commit exists
        if commit_exists(&prev_to_commit) {
            return Ok(RangeContext {
                from_commit: prev_to_commit,
                to_commit: head_sha,
                selection_mode: "incremental".to_string(),
                selection_detail: String::new(),
            });
        }
    }

    // Bootstrap: use recent commits
    let bootstrap_from = get_nth_ancestor(&head_sha, BOOTSTRAP_COMMIT_COUNT)?;
    Ok(RangeContext {
        from_commit: bootstrap_from,
        to_commit: head_sha,
        selection_mode: "bootstrap".to_string(),
        selection_detail: format!(
            "Last {} commits with non-.jules/ changes",
            BOOTSTRAP_COMMIT_COUNT
        ),
    })
}

/// Get the current HEAD SHA.
fn get_head_sha() -> Result<String, AppError> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::config_error("Failed to get HEAD sha"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get the Nth ancestor of a commit.
fn get_nth_ancestor(commit: &str, n: usize) -> Result<String, AppError> {
    let output = Command::new("git")
        .args(["rev-parse", &format!("{}~{}", commit, n)])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        // If we can't go back N commits, use the first commit in the ancestry of the given commit
        let first = Command::new("git")
            .args(["rev-list", "--max-parents=0", commit])
            .output()
            .map_err(|e| AppError::config_error(format!("Failed to get first commit: {}", e)))?;

        if first.status.success() {
            return Ok(String::from_utf8_lossy(&first.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string());
        }

        return Err(AppError::config_error("Failed to determine commit range"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if a commit exists in the repository.
fn commit_exists(sha: &str) -> bool {
    Command::new("git")
        .args(["cat-file", "-e", sha])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Extract to_commit from latest.yml content using proper YAML parsing.
fn extract_to_commit(content: &str) -> Option<String> {
    serde_yaml::from_str::<serde_yaml::Value>(content)
        .ok()
        .as_ref()
        .and_then(|data| data.get("range"))
        .and_then(|range| range.get("to_commit"))
        .and_then(|val| val.as_str())
        .filter(|s| !s.is_empty() && s.len() >= 7)
        .map(|s| s.to_string())
}

/// Check if there are any non-.jules/ changes in the range.
fn check_for_changes(range: &RangeContext) -> Result<bool, AppError> {
    let output = Command::new("git")
        .args([
            "diff",
            "--name-only",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::config_error("Failed to check for changes"));
    }

    let paths = String::from_utf8_lossy(&output.stdout);
    Ok(!paths.trim().is_empty())
}

/// Collect commits in the range, excluding .jules/-only commits.
fn collect_commits(range: &RangeContext) -> Result<(Vec<CommitInfo>, bool), AppError> {
    let output = Command::new("git")
        .args([
            "log",
            "--pretty=format:%H|%an|%aI|%s",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git log: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::config_error("Failed to get commit log"));
    }

    let mut commits = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() == 4 {
            commits.push(CommitInfo {
                sha: parts[0].to_string(),
                author: parts[1].to_string(),
                date: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }

    let truncated = commits.len() > MAX_COMMITS;
    if truncated {
        commits.truncate(MAX_COMMITS);
    }

    Ok((commits, truncated))
}

/// Collect changed paths in the range, excluding .jules/.
fn collect_changed_paths(range: &RangeContext) -> Result<(Vec<String>, bool), AppError> {
    let output = Command::new("git")
        .args([
            "diff",
            "--name-only",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::config_error("Failed to get changed paths"));
    }

    let mut paths: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let truncated = paths.len() > MAX_CHANGED_PATHS;
    if truncated {
        paths.truncate(MAX_CHANGED_PATHS);
    }

    Ok((paths, truncated))
}

/// Collect diffstat for the range using --numstat (machine-readable format).
fn collect_diffstat(range: &RangeContext) -> Result<DiffStat, AppError> {
    let output = Command::new("git")
        .args([
            "diff",
            "--numstat",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        return Ok(DiffStat::default());
    }

    let numstat_output = String::from_utf8_lossy(&output.stdout);
    Ok(parse_numstat(&numstat_output))
}

/// Parse git --numstat output.
/// Format: <insertions>\t<deletions>\t<path>
/// Binary files show "-" for insertions/deletions.
fn parse_numstat(output: &str) -> DiffStat {
    let mut stat = DiffStat::default();

    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            // Skip binary files (marked with "-")
            if parts[0] != "-" {
                stat.insertions += parts[0].parse::<u32>().unwrap_or(0);
            }
            if parts[1] != "-" {
                stat.deletions += parts[1].parse::<u32>().unwrap_or(0);
            }
            stat.files_changed += 1;
        }
    }

    stat
}

/// Build truncation note if needed.
fn build_truncation_note(commits_truncated: bool, paths_truncated: bool) -> String {
    match (commits_truncated, paths_truncated) {
        (true, true) => format!(
            "Commits truncated to {} and paths truncated to {}",
            MAX_COMMITS, MAX_CHANGED_PATHS
        ),
        (true, false) => format!("Commits truncated to {}", MAX_COMMITS),
        (false, true) => format!("Paths truncated to {}", MAX_CHANGED_PATHS),
        (false, false) => String::new(),
    }
}

/// Build the full Narrator prompt with git context injected.
fn build_narrator_prompt(jules_path: &Path, ctx: &GitContext) -> Result<String, AppError> {
    let base_prompt = assemble_single_role_prompt(jules_path, Layer::Narrator)?;

    // Build the git context section
    let mut context_section = String::new();
    context_section.push_str("\n---\n# Runner-Provided Git Context\n\n");

    context_section.push_str("## Range\n");
    context_section.push_str(&format!("- from_commit: {}\n", ctx.range.from_commit));
    context_section.push_str(&format!("- to_commit: {}\n", ctx.range.to_commit));
    context_section.push_str(&format!("- selection_mode: {}\n", ctx.range.selection_mode));
    if !ctx.range.selection_detail.is_empty() {
        context_section.push_str(&format!("- selection_detail: {}\n", ctx.range.selection_detail));
    }

    context_section.push_str("\n## Diffstat\n");
    context_section.push_str(&format!("- files_changed: {}\n", ctx.diffstat.files_changed));
    context_section.push_str(&format!("- insertions: {}\n", ctx.diffstat.insertions));
    context_section.push_str(&format!("- deletions: {}\n", ctx.diffstat.deletions));

    context_section.push_str(&format!("\n## Commits ({} total)\n", ctx.commits.len()));
    for commit in &ctx.commits {
        context_section.push_str(&format!(
            "- {} {} {} {}\n",
            &commit.sha[..7.min(commit.sha.len())],
            commit.date,
            commit.author,
            commit.message
        ));
    }

    context_section.push_str(&format!("\n## Changed Paths ({} total)\n", ctx.changed_paths.len()));
    for path in &ctx.changed_paths {
        context_section.push_str(&format!("- {}\n", path));
    }

    if !ctx.truncation_note.is_empty() {
        context_section.push_str(&format!("\n## Truncation\n{}\n", ctx.truncation_note));
    }

    Ok(format!("{}{}", base_prompt, context_section))
}

/// Execute a dry run, showing the assembled prompt and context.
fn execute_dry_run(
    jules_path: &Path,
    starting_branch: &str,
    ctx: &GitContext,
) -> Result<(), AppError> {
    println!("=== Dry Run: Narrator ===");
    println!("Starting branch: {}\n", starting_branch);

    println!("--- Git Context ---");
    println!(
        "Range: {}..{} ({})",
        ctx.range.from_commit, ctx.range.to_commit, ctx.range.selection_mode
    );
    println!(
        "Diffstat: {} files, +{} -{}",
        ctx.diffstat.files_changed, ctx.diffstat.insertions, ctx.diffstat.deletions
    );
    println!("Commits: {}", ctx.commits.len());
    println!("Changed paths: {}", ctx.changed_paths.len());
    if !ctx.truncation_note.is_empty() {
        println!("Truncation: {}", ctx.truncation_note);
    }

    println!("\n--- Prompt (base) ---");
    let prompt = assemble_single_role_prompt(jules_path, Layer::Narrator)?;
    println!("{}", prompt);

    Ok(())
}
