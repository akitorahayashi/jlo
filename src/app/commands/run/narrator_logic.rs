use crate::domain::AppError;
use chrono::DateTime;

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
            what: ".jules/exchange/changes.yml".to_string(),
            details: err.to_string(),
        })?;

    let created_at =
        data.get("created_at").and_then(|val| val.as_str()).filter(|s| !s.is_empty()).ok_or_else(
            || {
                AppError::Validation(
                    "changes.yml must contain non-empty created_at for incremental narrator runs"
                        .to_string(),
                )
            },
        )?;

    DateTime::parse_from_rfc3339(created_at).map_err(|err| {
        AppError::Validation(format!(
            "changes.yml created_at must be RFC3339: {} ({})",
            created_at, err
        ))
    })?;

    Ok(created_at.to_string())
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
}
