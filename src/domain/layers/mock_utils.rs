use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

use crate::domain::{AppError, IoErrorKind, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

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

/// Parse mock event ID from filename.
pub fn mock_event_id_from_path(path: &Path, mock_tag: &str) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let prefix = format!("mock-{}-", mock_tag);
    file_name.strip_prefix(&prefix)?.strip_suffix(".yml").map(ToString::to_string)
}

/// List files in directory matching the mock tag pattern.
pub fn list_mock_tagged_files<W: WorkspaceStore + ?Sized>(
    workspace: &W,
    dir: &Path,
    mock_tag: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let dir_str = dir.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!("Invalid directory path: {}", dir.display()))
    })?;

    let entries = match workspace.list_dir(dir_str) {
        Ok(entries) => entries,
        Err(AppError::Io { kind: IoErrorKind::NotFound, .. }) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let mut files: Vec<PathBuf> = entries
        .into_iter()
        .filter(|path| !workspace.is_dir(&path.to_string_lossy()))
        .filter(|path| mock_event_id_from_path(path, mock_tag).is_some())
        .collect();

    files.sort();
    Ok(files)
}

/// Service for executing mock workflows.
pub struct MockExecutionService<'a, G: ?Sized, H: ?Sized, W: ?Sized> {
    #[allow(dead_code)]
    pub jules_path: &'a Path,
    #[allow(dead_code)]
    pub config: &'a MockConfig,
    pub git: &'a G,
    pub github: &'a H,
    #[allow(dead_code)]
    pub workspace: &'a W,
}

impl<'a, G, H, W> MockExecutionService<'a, G, H, W>
where
    G: GitPort + ?Sized,
    H: GitHubPort + ?Sized,
    W: WorkspaceStore + ?Sized,
{
    pub fn new(
        jules_path: &'a Path,
        config: &'a MockConfig,
        git: &'a G,
        github: &'a H,
        workspace: &'a W,
    ) -> Self {
        Self { jules_path, config, git, github, workspace }
    }

    /// Fetch origin and checkout a base branch (detached HEAD).
    pub fn fetch_and_checkout_base(&self, base_branch: &str) -> Result<(), AppError> {
        self.git.fetch("origin")?;
        self.git.checkout_branch(&format!("origin/{}", base_branch), false)?;
        Ok(())
    }

    /// Checkout a new branch from the current HEAD.
    pub fn checkout_new_branch(&self, branch: &str) -> Result<(), AppError> {
        self.git.checkout_branch(branch, true)
    }

    /// Commit files and push the current branch.
    pub fn commit_and_push(
        &self,
        message: &str,
        files: &[&Path],
        branch: &str,
    ) -> Result<(), AppError> {
        self.git.commit_files(message, files)?;
        self.git.push_branch(branch, false)?;
        Ok(())
    }

    /// Create a pull request.
    pub fn create_pr(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<crate::ports::PullRequestInfo, AppError> {
        self.github.create_pull_request(head, base, title, body)
    }

    /// Write mock output to GITHUB_OUTPUT or stdout.
    pub fn finish(&self, output: &MockOutput) -> Result<(), AppError> {
        if std::env::var("GITHUB_OUTPUT").is_ok() {
            write_github_output(output).map_err(|e| {
                AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
            })?;
        } else {
            print_local(output);
        }
        Ok(())
    }
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
    }

    #[test]
    fn test_mock_event_id_from_path() {
        let mock_tag = "mock-run-123";
        let valid_path = std::path::Path::new("mock-mock-run-123-a1b2c3.yml");
        let invalid_path = std::path::Path::new("mock-other-tag-a1b2c3.yml");

        assert_eq!(mock_event_id_from_path(valid_path, mock_tag), Some("a1b2c3".to_string()));
        assert_eq!(mock_event_id_from_path(invalid_path, mock_tag), None);
    }
}
