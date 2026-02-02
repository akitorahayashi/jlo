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

/// Maximum number of commits to include in the bounded sample.
const MAX_COMMITS: usize = 50;
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
    pub stats: Stats,
    pub commits: Vec<CommitInfo>,
    pub truncation_note: String,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub commits_total: u32,
    pub commits_included: u32,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommitInfo {
    pub sha: String,
    pub subject: String,
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
            git_context.stats.files_changed
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

    // Count total commits in range (for truncation tracking)
    let commits_total = count_commits(&range)?;

    // Collect bounded commit samples
    let commits = collect_commits(&range)?;
    let commits_included = commits.len() as u32;

    // Collect diffstat
    let diffstat = collect_diffstat(&range)?;

    // Build stats
    let stats = Stats {
        commits_total,
        commits_included,
        files_changed: diffstat.files_changed,
        insertions: diffstat.insertions,
        deletions: diffstat.deletions,
    };

    // Build truncation note
    let truncation_note = if commits_total > commits_included {
        format!("Commits truncated to {} of {} total", commits_included, commits_total)
    } else {
        String::new()
    };

    Ok(Some(GitContext { range, stats, commits, truncation_note }))
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

/// Count total commits in the range, excluding .jules/-only commits.
fn count_commits(range: &RangeContext) -> Result<u32, AppError> {
    let output = Command::new("git")
        .args([
            "rev-list",
            "--count",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ])
        .output()
        .map_err(|e| AppError::config_error(format!("Failed to run git rev-list: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::config_error("Failed to count commits in range"));
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|e| AppError::config_error(format!("Failed to parse commit count: {}", e)))
}

/// Collect commits in the range (sha + subject only), excluding .jules/-only commits.
fn collect_commits(range: &RangeContext) -> Result<Vec<CommitInfo>, AppError> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("-{}", MAX_COMMITS),
            "--pretty=format:%H|%s",
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

    Ok(parse_commits_log(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_commits_log(output: &str) -> Vec<CommitInfo> {
    let mut commits = Vec::new();
    for line in output.lines() {
        if let Some((sha, subject)) = line.split_once('|') {
            commits.push(CommitInfo { sha: sha.to_string(), subject: subject.to_string() });
        }
    }
    commits
}

/// Internal struct for parsing diffstat (used to build Stats).
#[derive(Debug, Default, PartialEq, Eq)]
struct DiffStat {
    files_changed: u32,
    insertions: u32,
    deletions: u32,
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
        let mut parts = line.split('\t');
        if let (Some(insertions_str), Some(deletions_str), Some(_)) =
            (parts.next(), parts.next(), parts.next())
        {
            // Skip binary files (marked with "-")
            if insertions_str != "-" {
                stat.insertions += insertions_str.parse::<u32>().unwrap_or(0);
            }
            if deletions_str != "-" {
                stat.deletions += deletions_str.parse::<u32>().unwrap_or(0);
            }
            stat.files_changed += 1;
        }
    }

    stat
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
    let prompt = assemble_single_role_prompt(jules_path, Layer::Narrator)?;
    println!("{}", prompt);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commits_log_standard() {
        let input = "a1b2c3d|Initial commit\ne5f6g7h|Fix bug";
        let result = parse_commits_log(input);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].sha, "a1b2c3d");
        assert_eq!(result[0].subject, "Initial commit");
        assert_eq!(result[1].sha, "e5f6g7h");
        assert_eq!(result[1].subject, "Fix bug");
    }

    #[test]
    fn test_parse_commits_log_empty() {
        let input = "";
        let result = parse_commits_log(input);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_commits_log_malformed() {
        let input = "a1b2c3d|Subject\nmalformed_line\n|Empty sha\nsha_only|";
        let result = parse_commits_log(input);

        // "malformed_line" should be ignored (no |)
        // "|Empty sha" -> sha="", subject="Empty sha"
        // "sha_only|" -> sha="sha_only", subject=""

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].sha, "a1b2c3d");
        assert_eq!(result[0].subject, "Subject");

        assert_eq!(result[1].sha, "");
        assert_eq!(result[1].subject, "Empty sha");

        assert_eq!(result[2].sha, "sha_only");
        assert_eq!(result[2].subject, "");
    }

    #[test]
    fn test_parse_numstat_standard() {
        let input = "10\t5\tsrc/main.rs\n2\t0\tREADME.md";
        let result = parse_numstat(input);

        assert_eq!(result.files_changed, 2);
        assert_eq!(result.insertions, 12);
        assert_eq!(result.deletions, 5);
    }

    #[test]
    fn test_parse_numstat_binary() {
        let input = "-\t-\timage.png\n5\t2\tsrc/lib.rs";
        let result = parse_numstat(input);

        assert_eq!(result.files_changed, 2);
        assert_eq!(result.insertions, 5);
        assert_eq!(result.deletions, 2);
    }

    #[test]
    fn test_parse_numstat_empty() {
        let input = "";
        let result = parse_numstat(input);

        assert_eq!(result.files_changed, 0);
        assert_eq!(result.insertions, 0);
        assert_eq!(result.deletions, 0);
    }
}
