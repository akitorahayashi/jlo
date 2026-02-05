//! Canonical workflow output helper.
//!
//! Writes compact single-line JSON to stdout on success, and optionally
//! appends `json=<...>` to `GITHUB_OUTPUT` when set.

use serde::Serialize;
use std::io::Write;

use crate::domain::AppError;

/// Write workflow command output in canonical format.
///
/// - Writes compact single-line JSON to stdout.
/// - Appends `json=<same JSON>` to `GITHUB_OUTPUT` file if the env var is set.
///
/// # Errors
/// Returns an error if JSON serialization fails or file I/O fails.
pub fn write_workflow_output<T: Serialize>(output: &T) -> Result<(), AppError> {
    // Serialize to compact single-line JSON (no pretty printing)
    let json = serde_json::to_string(output).map_err(|e| {
        AppError::InternalError(format!("Failed to serialize workflow output: {}", e))
    })?;

    // Sanity check: ensure no embedded newlines
    debug_assert!(!json.contains('\n'), "workflow output JSON must be single-line");

    // Write to stdout
    println!("{}", json);

    // Write to GITHUB_OUTPUT if set
    if let Ok(path) = std::env::var("GITHUB_OUTPUT") {
        let mut file =
            std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| {
                AppError::InternalError(format!("Failed to open GITHUB_OUTPUT: {}", e))
            })?;

        // Write as single line: json=<value>
        // Value must not contain newlines (enforced above)
        writeln!(file, "json={}", json).map_err(|e| {
            AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::fs;
    use tempfile::NamedTempFile;

    #[derive(Serialize)]
    struct TestOutput {
        schema_version: u32,
        ok: bool,
    }

    #[test]
    fn output_is_single_line_json() {
        let output = TestOutput { schema_version: 1, ok: true };
        let json = serde_json::to_string(&output).unwrap();

        assert!(!json.contains('\n'), "JSON output must not contain newlines");
        assert!(!json.contains("  "), "JSON output must not be pretty-printed");
    }

    #[test]
    fn github_output_contains_single_line_value() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_string_lossy().to_string();

        // Set GITHUB_OUTPUT for this test scope
        // SAFETY: Tests run in serial, env var manipulation is isolated
        unsafe {
            std::env::set_var("GITHUB_OUTPUT", &path);
        }

        let output = TestOutput { schema_version: 1, ok: true };
        write_workflow_output(&output).unwrap();

        // Unset to avoid affecting other tests
        // SAFETY: Tests run in serial, env var manipulation is isolated
        unsafe {
            std::env::remove_var("GITHUB_OUTPUT");
        }

        let contents = fs::read_to_string(temp_file.path()).unwrap();
        let lines: Vec<&str> = contents.lines().collect();

        assert_eq!(lines.len(), 1, "GITHUB_OUTPUT should contain exactly one line");
        assert!(lines[0].starts_with("json="), "Line should start with json=");

        // The value after json= should be valid JSON without newlines
        let value = lines[0].strip_prefix("json=").unwrap();
        assert!(!value.contains('\n'), "Value must not contain newlines");

        // Validate it's parseable JSON
        let parsed: serde_json::Value = serde_json::from_str(value).unwrap();
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["ok"], true);
    }

    #[test]
    fn no_github_output_when_env_not_set() {
        // Ensure env var is not set
        // SAFETY: Tests run in serial, env var manipulation is isolated
        unsafe {
            std::env::remove_var("GITHUB_OUTPUT");
        }

        let output = TestOutput { schema_version: 1, ok: true };
        // Should not panic or error when GITHUB_OUTPUT is not set
        let result = write_workflow_output(&output);
        assert!(result.is_ok());
    }
}
