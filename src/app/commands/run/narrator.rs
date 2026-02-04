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
use crate::services::adapters::jules_client_http::HttpJulesClient;

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

#[derive(Debug)]
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

    let repo_root = jules_path.parent().unwrap_or(Path::new("."));

    let range = determine_range(&latest_path, repo_root)?;

    // Check if there are any non-excluded changes in the range
    let has_changes = check_for_changes(&range, repo_root)?;
    if !has_changes {
        return Ok(None);
    }

    // Count total commits in range (for truncation tracking)
    let commits_total = count_commits(&range, repo_root)?;

    // Collect bounded commit samples
    let commits = collect_commits(&range, repo_root)?;
    let commits_included = commits.len() as u32;

    // Collect diffstat
    let diffstat = collect_diffstat(&range, repo_root)?;

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
fn determine_range(latest_path: &Path, repo_root: &Path) -> Result<RangeContext, AppError> {
    let head_sha = get_head_sha(repo_root)?;

    if latest_path.exists()
        && let Ok(content) = fs::read_to_string(latest_path)
        && let Some(prev_to_commit) = extract_to_commit(&content)
    {
        // Verify the commit exists
        if commit_exists(&prev_to_commit, repo_root) {
            return Ok(RangeContext {
                from_commit: prev_to_commit,
                to_commit: head_sha,
                selection_mode: "incremental".to_string(),
                selection_detail: String::new(),
            });
        }
    }

    // Bootstrap: use recent commits
    let bootstrap_from = get_nth_ancestor(&head_sha, BOOTSTRAP_COMMIT_COUNT, repo_root)?;
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
fn get_head_sha(repo_root: &Path) -> Result<String, AppError> {
    run_git(&["rev-parse", "HEAD"], Some(repo_root))
}

/// Get the Nth ancestor of a commit.
fn get_nth_ancestor(commit: &str, n: usize, repo_root: &Path) -> Result<String, AppError> {
    match run_git(&["rev-parse", &format!("{}~{}", commit, n)], Some(repo_root)) {
        Ok(sha) => Ok(sha),
        Err(_) => {
            // If we can't go back N commits, use the first commit in the ancestry of the given commit
            let first = run_git(&["rev-list", "--max-parents=0", commit], Some(repo_root))?;
            Ok(first.lines().next().unwrap_or("").to_string())
        }
    }
}

/// Check if a commit exists in the repository.
fn commit_exists(sha: &str, repo_root: &Path) -> bool {
    run_git(&["cat-file", "-e", sha], Some(repo_root)).is_ok()
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
fn check_for_changes(range: &RangeContext, repo_root: &Path) -> Result<bool, AppError> {
    let output = run_git(
        &[
            "diff",
            "--name-only",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ],
        Some(repo_root),
    )?;

    Ok(!output.trim().is_empty())
}

/// Count total commits in the range, excluding .jules/-only commits.
fn count_commits(range: &RangeContext, repo_root: &Path) -> Result<u32, AppError> {
    let output = run_git(
        &[
            "rev-list",
            "--count",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ],
        Some(repo_root),
    )?;

    output.trim().parse().map_err(|e| AppError::ParseError {
        what: "commit count".to_string(),
        details: format!("Value: '{}', Error: {}", output, e),
    })
}

/// Collect commits in the range (sha + subject only), excluding .jules/-only commits.
fn collect_commits(range: &RangeContext, repo_root: &Path) -> Result<Vec<CommitInfo>, AppError> {
    let output = run_git(
        &[
            "log",
            &format!("-{}", MAX_COMMITS),
            "--pretty=format:%H|%s",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ],
        Some(repo_root),
    )?;

    let mut commits = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() == 2 {
            commits.push(CommitInfo { sha: parts[0].to_string(), subject: parts[1].to_string() });
        }
    }

    Ok(commits)
}

/// Internal struct for parsing diffstat (used to build Stats).
#[derive(Debug, Default)]
struct DiffStat {
    files_changed: u32,
    insertions: u32,
    deletions: u32,
}

/// Collect diffstat for the range using --numstat (machine-readable format).
fn collect_diffstat(range: &RangeContext, repo_root: &Path) -> Result<DiffStat, AppError> {
    let output = run_git(
        &[
            "diff",
            "--numstat",
            &format!("{}..{}", range.from_commit, range.to_commit),
            "--",
            ".",
            ":(exclude).jules",
        ],
        Some(repo_root),
    );

    // If git diff fails (e.g. fatal: ambiguous argument), we propagate the error
    // previously it returned Ok(DiffStat::default()), but generic error is bad.
    // If specific recovery is needed, it should be done here.
    // Assuming for now that failure to get diffstat is an error.
    let output = output?;

    Ok(parse_numstat(&output))
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

/// Helper to run git commands and map errors to AppError::GitError.
fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String, AppError> {
    let mut command = Command::new("git");
    command.args(args);
    if let Some(path) = cwd {
        command.current_dir(path);
    }

    let output = command.output().map_err(|e| AppError::GitError {
        command: format!("git {}", args.join(" ")),
        details: e.to_string(),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::GitError {
            command: format!("git {}", args.join(" ")),
            details: if stderr.is_empty() { "Unknown error".to_string() } else { stderr },
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Build the full Narrator prompt with git context injected.
fn build_narrator_prompt(jules_path: &Path, ctx: &GitContext) -> Result<String, AppError> {
    let base_prompt = assemble_single_role_prompt(jules_path, Layer::Narrators)?;

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
    let prompt = assemble_single_role_prompt(jules_path, Layer::Narrators)?;
    println!("{}", prompt);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    #[test]
    fn test_git_error_propagation() {
        // Create a temp dir that is NOT a git repo
        let temp = TempDir::new().unwrap();
        let path = temp.path();

        // Attempt to run get_head_sha using the temp dir as repo root
        // This should fail because it's not a git repo
        let result = get_head_sha(path);

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::GitError { command, details } => {
                assert!(command.contains("git rev-parse HEAD"));
                assert!(!details.is_empty());
            }
            err => panic!("Expected GitError, got {:?}", err),
        }
    }

    #[test]
    fn test_parse_numstat() {
        let output = "1\t2\tfile1.rs\n10\t0\tfile2.rs\n-\t-\tbinary.bin";
        let stats = parse_numstat(output);

        assert_eq!(stats.files_changed, 3);
        assert_eq!(stats.insertions, 11);
        assert_eq!(stats.deletions, 2);
    }

    #[test]
    fn test_parse_numstat_empty() {
        let stats = parse_numstat("");
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_parse_numstat_malformed() {
        let output = "invalid line\n1\t1\tgood.rs";
        let stats = parse_numstat(output);
        // Should handle gracefully
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.insertions, 1);
        assert_eq!(stats.deletions, 1);
    }

    #[test]
    fn test_extract_to_commit_valid() {
        let content = r#"
range:
  to_commit: "abcdef123456"
"#;
        let commit = extract_to_commit(content);
        assert_eq!(commit, Some("abcdef123456".to_string()));
    }

    #[test]
    fn test_extract_to_commit_missing() {
        let content = r#"
range:
  other: "value"
"#;
        assert_eq!(extract_to_commit(content), None);
    }

    #[test]
    fn test_extract_to_commit_invalid_yaml() {
        let content = "::invalid yaml";
        assert_eq!(extract_to_commit(content), None);
    }

    #[test]
    fn test_extract_to_commit_short() {
        let content = r#"
range:
  to_commit: "abc"
"#;
        // Filter expects >= 7 chars
        assert_eq!(extract_to_commit(content), None);
    }
}
