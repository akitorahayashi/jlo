use crate::domain::AppError;
use crate::ports::{CommitInfo, DiffStat};

/// Maximum number of commits to include in the bounded sample.
pub const MAX_COMMITS: usize = 50;
/// Number of commits to use for bootstrap when no prior summary exists.
pub const BOOTSTRAP_COMMIT_COUNT: usize = 20;

/// Range selection context for Narrator.
#[derive(Debug, PartialEq)]
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

#[derive(Debug, Default)]
pub struct Stats {
    pub commits_total: u32,
    pub commits_included: u32,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug)]
pub struct GitContext {
    pub range: RangeContext,
    pub stats: Stats,
    pub commits: Vec<CommitInfo>,
    pub truncation_note: String,
}

#[derive(Debug)]
pub struct NarratorGitData {
    pub range: RangeContext,
    pub has_changes: bool,
    pub commits_total: u32,
    pub commits: Vec<CommitInfo>,
    pub diffstat: DiffStat,
}

/// Analyze collected git data and build context if applicable.
pub fn analyze_git_context(data: NarratorGitData) -> Option<GitContext> {
    if !data.has_changes {
        return None;
    }

    let stats = Stats {
        commits_total: data.commits_total,
        commits_included: data.commits.len() as u32,
        files_changed: data.diffstat.files_changed,
        insertions: data.diffstat.insertions,
        deletions: data.diffstat.deletions,
    };

    Some(build_git_context(data.range, stats, data.commits))
}

/// Determine the commit range strategy based on inputs.
pub fn determine_range_strategy(
    head_sha: &str,
    latest_yml_content: Option<&str>,
    check_commit_exists: impl Fn(&str) -> bool,
    get_bootstrap_commit: impl Fn(&str, usize) -> Result<String, AppError>,
) -> Result<RangeContext, AppError> {
    if let Some(content) = latest_yml_content
        && let Some(prev_to_commit) = extract_to_commit(content)
    {
        // Verify the commit exists
        if check_commit_exists(&prev_to_commit) {
            return Ok(RangeContext {
                from_commit: prev_to_commit,
                to_commit: head_sha.to_string(),
                selection_mode: "incremental".to_string(),
                selection_detail: String::new(),
            });
        }
    }

    // Bootstrap: use recent commits
    let bootstrap_from = get_bootstrap_commit(head_sha, BOOTSTRAP_COMMIT_COUNT)?;
    Ok(RangeContext {
        from_commit: bootstrap_from,
        to_commit: head_sha.to_string(),
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

pub fn build_git_context(
    range: RangeContext,
    stats: Stats,
    commits: Vec<CommitInfo>,
) -> GitContext {
    let truncation_note = if stats.commits_total > stats.commits_included {
        format!("Commits truncated to {} of {} total", stats.commits_included, stats.commits_total)
    } else {
        String::new()
    };

    GitContext { range, stats, commits, truncation_note }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_determine_range_strategy_incremental() {
        let head = "head_sha";
        let latest = r#"
range:
  to_commit: "prev_sha"
"#;
        let result = determine_range_strategy(
            head,
            Some(latest),
            |sha| sha == "prev_sha",
            |_, _| panic!("Should not bootstrap"),
        )
        .unwrap();

        assert_eq!(result.selection_mode, "incremental");
        assert_eq!(result.from_commit, "prev_sha");
        assert_eq!(result.to_commit, head);
    }

    #[test]
    fn test_determine_range_strategy_bootstrap_no_file() {
        let head = "head_sha";
        let result =
            determine_range_strategy(head, None, |_| false, |_, _| Ok("bootstrap_sha".to_string()))
                .unwrap();

        assert_eq!(result.selection_mode, "bootstrap");
        assert_eq!(result.from_commit, "bootstrap_sha");
        assert_eq!(result.to_commit, head);
    }

    #[test]
    fn test_determine_range_strategy_bootstrap_invalid_commit() {
        let head = "head_sha";
        let latest = r#"
range:
  to_commit: "prev_sha"
"#;
        let result = determine_range_strategy(
            head,
            Some(latest),
            |_| false, // prev_sha does not exist
            |_, _| Ok("bootstrap_sha".to_string()),
        )
        .unwrap();

        assert_eq!(result.selection_mode, "bootstrap");
        assert_eq!(result.from_commit, "bootstrap_sha");
    }

    #[test]
    fn test_build_git_context_truncation() {
        let range = RangeContext {
            from_commit: "a".into(),
            to_commit: "b".into(),
            selection_mode: "mode".into(),
            selection_detail: "".into(),
        };
        let stats = Stats {
            commits_total: 100,
            commits_included: 50,
            files_changed: 1,
            insertions: 1,
            deletions: 1,
        };
        let commits = vec![]; // Empty for test

        let ctx = build_git_context(range, stats, commits);
        assert!(ctx.truncation_note.contains("truncated to 50 of 100"));
    }
}
