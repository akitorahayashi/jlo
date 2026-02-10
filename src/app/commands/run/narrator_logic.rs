use crate::domain::AppError;
use crate::ports::{CommitInfo, DiffStat};
use chrono::DateTime;

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
    /// RFC3339 timestamp used as incremental cursor, if present.
    pub changes_since: Option<String>,
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
    get_bootstrap_commit: impl Fn(&str, usize) -> Result<String, AppError>,
    get_commit_before_time: impl Fn(&str, &str) -> Result<Option<String>, AppError>,
) -> Result<RangeContext, AppError> {
    if let Some(content) = latest_yml_content {
        let previous_created_at = extract_created_at(content)?;
        if let Some(base_commit) = get_commit_before_time(head_sha, &previous_created_at)? {
            return Ok(RangeContext {
                from_commit: base_commit,
                to_commit: head_sha.to_string(),
                selection_mode: "incremental".to_string(),
                selection_detail: String::new(),
                changes_since: Some(previous_created_at),
            });
        }

        // Explicit fallback: no commit exists before the cursor time.
        let bootstrap_from = get_bootstrap_commit(head_sha, BOOTSTRAP_COMMIT_COUNT)?;
        return Ok(RangeContext {
            from_commit: bootstrap_from,
            to_commit: head_sha.to_string(),
            selection_mode: "bootstrap".to_string(),
            selection_detail: format!(
                "Fallback bootstrap: no commit found before previous created_at ({})",
                previous_created_at
            ),
            changes_since: Some(previous_created_at),
        });
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
        changes_since: None,
    })
}

/// Extract and validate created_at from latest.yml content.
fn extract_created_at(content: &str) -> Result<String, AppError> {
    let data =
        serde_yaml::from_str::<serde_yaml::Value>(content).map_err(|err| AppError::ParseError {
            what: ".jules/changes/latest.yml".to_string(),
            details: err.to_string(),
        })?;

    let created_at =
        data.get("created_at").and_then(|val| val.as_str()).filter(|s| !s.is_empty()).ok_or_else(
            || {
                AppError::Validation(
                    "latest.yml must contain non-empty created_at for incremental narrator runs"
                        .to_string(),
                )
            },
        )?;

    DateTime::parse_from_rfc3339(created_at).map_err(|err| {
        AppError::Validation(format!(
            "latest.yml created_at must be RFC3339: {} ({})",
            created_at, err
        ))
    })?;

    Ok(created_at.to_string())
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
    fn test_extract_created_at_valid() {
        let content = r#"
created_at: "2026-02-05T00:00:00Z"
"#;
        let created_at = extract_created_at(content).unwrap();
        assert_eq!(created_at, "2026-02-05T00:00:00Z");
    }

    #[test]
    fn test_determine_range_strategy_incremental() {
        let head = "head_sha";
        let latest = r#"
created_at: "2026-02-05T00:00:00Z"
"#;
        let result = determine_range_strategy(
            head,
            Some(latest),
            |_, _| panic!("Should not bootstrap"),
            |_, timestamp| {
                assert_eq!(timestamp, "2026-02-05T00:00:00Z");
                Ok(Some("base_sha".to_string()))
            },
        )
        .unwrap();

        assert_eq!(result.selection_mode, "incremental");
        assert_eq!(result.from_commit, "base_sha");
        assert_eq!(result.to_commit, head);
        assert_eq!(result.changes_since.as_deref(), Some("2026-02-05T00:00:00Z"));
    }

    #[test]
    fn test_determine_range_strategy_bootstrap_no_file() {
        let head = "head_sha";
        let result = determine_range_strategy(
            head,
            None,
            |_, _| Ok("bootstrap_sha".to_string()),
            |_, _| panic!("Should not resolve incremental cursor"),
        )
        .unwrap();

        assert_eq!(result.selection_mode, "bootstrap");
        assert_eq!(result.from_commit, "bootstrap_sha");
        assert_eq!(result.to_commit, head);
        assert_eq!(result.changes_since, None);
    }

    #[test]
    fn test_determine_range_strategy_bootstrap_when_no_commit_before_cursor() {
        let head = "head_sha";
        let latest = r#"
created_at: "2026-02-05T00:00:00Z"
"#;
        let result = determine_range_strategy(
            head,
            Some(latest),
            |_, _| Ok("bootstrap_sha".to_string()),
            |_, _| Ok(None),
        )
        .unwrap();

        assert_eq!(result.selection_mode, "bootstrap");
        assert_eq!(result.from_commit, "bootstrap_sha");
        assert!(result.selection_detail.contains("Fallback bootstrap"));
        assert_eq!(result.changes_since.as_deref(), Some("2026-02-05T00:00:00Z"));
    }

    #[test]
    fn test_extract_created_at_missing_is_error() {
        let content = r#"
range:
  to_commit: "abcdef123456"
"#;
        let err = extract_created_at(content).unwrap_err();
        assert!(err.to_string().contains("created_at"));
    }

    #[test]
    fn test_extract_created_at_invalid_format_is_error() {
        let content = r#"
created_at: "2026-02-05 00:00:00"
"#;
        let err = extract_created_at(content).unwrap_err();
        assert!(err.to_string().contains("RFC3339"));
    }

    #[test]
    fn test_build_git_context_truncation() {
        let range = RangeContext {
            from_commit: "a".into(),
            to_commit: "b".into(),
            selection_mode: "mode".into(),
            selection_detail: "".into(),
            changes_since: Some("2026-02-05T00:00:00Z".into()),
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
