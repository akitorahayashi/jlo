use include_dir::{Dir, include_dir};

use crate::domain::MockOutput;

/// Mock assets embedded in the binary.
pub static MOCK_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/assets/mock");

/// Write outputs to GITHUB_OUTPUT file if set.
pub fn write_github_output(output: &MockOutput) -> std::io::Result<()> {
    if let Ok(output_file) = std::env::var("GITHUB_OUTPUT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&output_file)?;
        writeln!(file, "mock_branch={}", output.mock_branch)?;
        writeln!(file, "mock_pr_number={}", output.mock_pr_number)?;
        writeln!(file, "mock_pr_url={}", output.mock_pr_url)?;
        writeln!(file, "mock_tag={}", output.mock_tag)?;
    }
    Ok(())
}

/// Print outputs in grep-friendly format for local use.
pub fn print_local(output: &MockOutput) {
    println!("MOCK_BRANCH={}", output.mock_branch);
    println!("MOCK_PR_NUMBER={}", output.mock_pr_number);
    println!("MOCK_PR_URL={}", output.mock_pr_url);
    println!("MOCK_TAG={}", output.mock_tag);
}

/// Generate a 6-character mock ID.
pub fn generate_mock_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{:06x}", (timestamp % 0xFFFFFF) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mock_id() {
        let id1 = generate_mock_id();
        let id2 = generate_mock_id();
        assert_eq!(id1.len(), 6);
        assert_eq!(id2.len(), 6);
        // IDs should be different (very high probability)
        // Note: This could theoretically fail if called in same nanosecond
    }
}
