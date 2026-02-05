//! Narrator layer execution (single-role, not issue-driven).
//!
//! The Narrator produces `.jules/changes/latest.yml` summarizing recent codebase changes.

use std::path::Path;

use super::RunResult;
use super::config::load_config;
use super::prompt::assemble_single_role_prompt;
use crate::domain::{AppError, Layer};
use crate::ports::{
    AutomationMode, CommitInfo, GitPort, JulesClient, SessionRequest, WorkspaceStore,
};
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

/// Execute the Narrator layer.
pub fn execute<G, W>(
    jules_path: &Path,
    dry_run: bool,
    branch: Option<&str>,
    is_ci: bool,
    git: &G,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    G: GitPort,
    W: WorkspaceStore,
{
    let config = load_config(jules_path, workspace)?;

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
                dry_run,
                sessions: vec![],
            });
        }
    };

    if dry_run {
        execute_dry_run(jules_path, &starting_branch, &git_context, workspace)?;
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
    let source = git.get_remote_url("origin")?;
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
fn collect_git_context<G, W>(
    jules_path: &Path,
    git: &G,
    workspace: &W,
) -> Result<Option<GitContext>, AppError>
where
    G: GitPort,
    W: WorkspaceStore,
{
    // Construct path to latest.yml relative to workspace root or absolute
    let latest_path = jules_path.join("changes/latest.yml");
    let latest_path_str = latest_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Jules path contains invalid unicode".to_string()))?;

    let range = determine_range(latest_path_str, git, workspace)?;

    // Check if there are any non-excluded changes in the range
    let pathspec = &[".", ":(exclude).jules"];
    let has_changes = git.has_changes(&range.from_commit, &range.to_commit, pathspec)?;

    if !has_changes {
        return Ok(None);
    }

    // Count total commits in range (for truncation tracking)
    let commits_total = git.count_commits(&range.from_commit, &range.to_commit, pathspec)?;

    // Collect bounded commit samples
    let commits =
        git.collect_commits(&range.from_commit, &range.to_commit, pathspec, MAX_COMMITS)?;
    let commits_included = commits.len() as u32;

    // Collect diffstat
    let diffstat = git.get_diffstat(&range.from_commit, &range.to_commit, pathspec)?;

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

    if let Ok(content) = workspace.read_file(latest_path)
        && let Some(prev_to_commit) = extract_to_commit(&content)
    {
        // Verify the commit exists
        if git.commit_exists(&prev_to_commit) {
            return Ok(RangeContext {
                from_commit: prev_to_commit,
                to_commit: head_sha,
                selection_mode: "incremental".to_string(),
                selection_detail: String::new(),
            });
        }
    }

    // Bootstrap: use recent commits
    let bootstrap_from = git.get_nth_ancestor(&head_sha, BOOTSTRAP_COMMIT_COUNT)?;
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

/// Build the full Narrator prompt with git context injected.
fn build_narrator_prompt(
    jules_path: &Path,
    ctx: &GitContext,
    workspace: &impl WorkspaceStore,
) -> Result<String, AppError> {
    let base_prompt = assemble_single_role_prompt(jules_path, Layer::Narrators, workspace)?;

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
    workspace: &impl WorkspaceStore,
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
    let prompt = assemble_single_role_prompt(jules_path, Layer::Narrators, workspace)?;
    println!("{}", prompt);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RoleId;
    use crate::ports::{DiffStat, DiscoveredRole, ScaffoldFile};
    use std::collections::HashMap;

    struct MockGitPort {
        head_sha: String,
        changes: bool,
    }

    impl MockGitPort {
        fn new() -> Self {
            Self { head_sha: "head123".to_string(), changes: true }
        }
    }

    impl GitPort for MockGitPort {
        fn get_head_sha(&self) -> Result<String, AppError> {
            Ok(self.head_sha.clone())
        }
        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok("main".to_string())
        }
        fn get_remote_url(&self, _name: &str) -> Result<String, AppError> {
            Ok("https://github.com/owner/repo.git".to_string())
        }
        fn commit_exists(&self, _sha: &str) -> bool {
            true
        }
        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
            Ok("ancestor123".to_string())
        }
        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            Ok(self.changes)
        }
        fn count_commits(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<u32, AppError> {
            Ok(10)
        }
        fn collect_commits(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
            _limit: usize,
        ) -> Result<Vec<CommitInfo>, AppError> {
            Ok(vec![CommitInfo { sha: "abc".to_string(), subject: "test".to_string() }])
        }
        fn get_diffstat(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<DiffStat, AppError> {
            Ok(DiffStat { files_changed: 1, insertions: 10, deletions: 5 })
        }
        fn run_command(&self, _args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
            Ok("".to_string())
        }
        fn checkout_branch(&self, _branch: &str, _create: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn push_branch(&self, _branch: &str, _force: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            Ok("newsha".to_string())
        }
        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    struct MockWorkspaceStore {
        files: HashMap<String, String>,
    }

    impl MockWorkspaceStore {
        fn new() -> Self {
            Self { files: HashMap::new() }
        }
        fn with_config(mut self) -> Self {
            self.files.insert(
                ".jules/config.toml".to_string(),
                r#"
[run]
default_branch = "main"
[jules]
api_url = "http://example.com"
"#
                .to_string(),
            );
            self
        }
    }

    impl WorkspaceStore for MockWorkspaceStore {
        fn exists(&self) -> bool {
            true
        }
        fn jules_path(&self) -> std::path::PathBuf {
            std::path::PathBuf::from(".jules")
        }
        fn read_file(&self, path: &str) -> Result<String, AppError> {
            self.files
                .get(path)
                .cloned()
                .ok_or(AppError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "not found")))
        }
        fn create_structure(&self, _scaffold_files: &[ScaffoldFile]) -> Result<(), AppError> {
            Ok(())
        }
        fn write_version(&self, _version: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn read_version(&self) -> Result<Option<String>, AppError> {
            Ok(None)
        }
        fn role_exists_in_layer(&self, _layer: Layer, _role_id: &RoleId) -> bool {
            true
        }
        fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
            Ok(vec![])
        }
        fn find_role_fuzzy(&self, _query: &str) -> Result<Option<DiscoveredRole>, AppError> {
            Ok(None)
        }
        fn role_path(&self, _role: &DiscoveredRole) -> Option<std::path::PathBuf> {
            None
        }
        fn scaffold_role_in_layer(
            &self,
            _layer: Layer,
            _role_id: &RoleId,
            _role_yaml: &str,
        ) -> Result<(), AppError> {
            Ok(())
        }
        fn create_workstream(&self, _name: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn list_workstreams(&self) -> Result<Vec<String>, AppError> {
            Ok(vec![])
        }
        fn workstream_exists(&self, _name: &str) -> bool {
            true
        }
        fn write_file(&self, _path: &str, _content: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn remove_file(&self, _path: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn create_dir_all(&self, _path: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn copy_file(&self, _src: &str, _dst: &str) -> Result<u64, AppError> {
            Ok(0)
        }
        fn resolve_path(&self, path: &str) -> std::path::PathBuf {
            std::path::PathBuf::from(path)
        }
        fn canonicalize(&self, path: &str) -> Result<std::path::PathBuf, AppError> {
            Ok(std::path::PathBuf::from(path))
        }
    }

    #[test]
    fn test_execute_no_changes() {
        let mut git = MockGitPort::new();
        git.changes = false;
        let ws = MockWorkspaceStore::new().with_config();

        let result = execute(Path::new(".jules"), false, None, false, &git, &ws).unwrap();

        assert_eq!(result.roles, vec!["narrator"]);
        assert!(result.sessions.is_empty());
    }

    #[test]
    fn test_execute_dry_run() {
        let git = MockGitPort::new();
        let mut ws = MockWorkspaceStore::new().with_config();
        // Setup prompt files
        ws.files
            .insert(".jules/roles/narrator/prompt.yml".to_string(), "role: narrator".to_string());
        ws.files.insert(
            ".jules/roles/narrator/contracts.yml".to_string(),
            "layer: narrator\nconstraints: []".to_string(),
        );
        ws.files.insert(
            ".jules/roles/narrator/prompt_assembly.yml".to_string(),
            "schema_version: 1\nlayer: narrator\nruntime_context: {}\nincludes: []".to_string(),
        );

        let result = execute(Path::new(".jules"), true, None, false, &git, &ws).unwrap();

        assert!(result.dry_run);
        assert!(result.sessions.is_empty());
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
