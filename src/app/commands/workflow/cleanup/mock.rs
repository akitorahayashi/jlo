//! Workflow cleanup mock command implementation.
//!
//! Closes mock PRs, deletes mock branches, removes mock-tagged files from jules branch.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde::Serialize;

use crate::domain::AppError;
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};
use crate::services::adapters::git_command::GitCommandAdapter;
use crate::services::adapters::github_command::GitHubCommandAdapter;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

/// Options for workflow cleanup mock command.
#[derive(Debug, Clone)]
pub struct WorkflowCleanupMockOptions {
    /// Mock tag to identify artifacts.
    pub mock_tag: String,
    /// PR numbers to close (optional, discovered if not provided).
    pub pr_numbers_json: Option<Vec<u64>>,
    /// Branches to delete (optional, discovered if not provided).
    pub branches_json: Option<Vec<String>>,
}

/// Output of workflow cleanup mock command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowCleanupMockOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Number of PRs closed.
    pub closed_prs_count: usize,
    /// Number of branches deleted.
    pub deleted_branches_count: usize,
    /// Number of mock files deleted from jules branch.
    pub deleted_files_count: usize,
}

/// Execute cleanup mock command.
pub fn execute(options: WorkflowCleanupMockOptions) -> Result<WorkflowCleanupMockOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    // Require GH_TOKEN and GITHUB_REPOSITORY.
    if std::env::var("GH_TOKEN").is_err() {
        return Err(AppError::Validation(
            "GH_TOKEN environment variable is required for cleanup mock".to_string(),
        ));
    }
    if std::env::var("GITHUB_REPOSITORY").is_err() {
        return Err(AppError::Validation(
            "GITHUB_REPOSITORY environment variable is required for cleanup mock".to_string(),
        ));
    }

    // Validate mock tag
    if !options.mock_tag.contains("mock") {
        return Err(AppError::Validation("mock_tag must contain 'mock' substring".to_string()));
    }

    let repository = std::env::var("GITHUB_REPOSITORY").map_err(|_| {
        AppError::Validation("GITHUB_REPOSITORY environment variable is required".to_string())
    })?;

    let root = workspace_root(&workspace)?;
    let git = GitCommandAdapter::new(root);
    let github = GitHubCommandAdapter::new();
    ensure_jules_branch_checked_out(&git)?;

    let current_branch = git.get_current_branch()?;
    if !current_branch.starts_with("jules") {
        return Err(AppError::Validation(format!(
            "Mock cleanup must run on a jules branch, current branch is '{}'",
            current_branch
        )));
    }

    let pr_numbers = match options.pr_numbers_json {
        Some(values) => sort_and_dedup(values),
        None => discover_mock_pr_numbers(&repository, &options.mock_tag)?,
    };

    let branches = match options.branches_json {
        Some(values) => sort_and_dedup(values),
        None => discover_mock_branches(&git, &options.mock_tag)?,
    };

    let closed_prs_count = close_pull_requests(&github, &pr_numbers)?;
    let deleted_branches_count = delete_remote_branches(&github, &branches)?;
    let deleted_files_count = delete_mock_files(&workspace, &git, &options.mock_tag)?;

    eprintln!(
        "Cleaned mock artifacts for tag '{}': {} PRs, {} branches, {} files",
        options.mock_tag, closed_prs_count, deleted_branches_count, deleted_files_count
    );

    Ok(WorkflowCleanupMockOutput {
        schema_version: 1,
        closed_prs_count,
        deleted_branches_count,
        deleted_files_count,
    })
}

fn ensure_jules_branch_checked_out(git: &GitCommandAdapter) -> Result<(), AppError> {
    git.fetch("origin")?;

    match git.run_command(&["checkout", "jules"], None) {
        Ok(_) => Ok(()),
        Err(_) => {
            git.run_command(&["checkout", "-b", "jules", "origin/jules"], None)?;
            Ok(())
        }
    }
}

fn close_pull_requests(
    github: &GitHubCommandAdapter,
    pr_numbers: &[u64],
) -> Result<usize, AppError> {
    let mut closed_count = 0;
    for pr_number in pr_numbers {
        github.close_pull_request(*pr_number)?;
        closed_count += 1;
    }
    Ok(closed_count)
}

fn delete_remote_branches(
    github: &GitHubCommandAdapter,
    branches: &[String],
) -> Result<usize, AppError> {
    let mut deleted_count = 0;
    for branch in branches {
        github.delete_branch(branch)?;
        deleted_count += 1;
    }
    Ok(deleted_count)
}

fn delete_mock_files(
    workspace: &FilesystemWorkspaceStore,
    git: &GitCommandAdapter,
    mock_tag: &str,
) -> Result<usize, AppError> {
    let root = workspace_root(workspace)?;
    let jules_path = workspace.jules_path();

    let files = collect_mock_files(&jules_path, mock_tag)?;
    if files.is_empty() {
        return Ok(0);
    }

    for file in &files {
        let relative = to_repo_relative(&root, file);
        git.run_command(&["rm", "-f", "--ignore-unmatch", "--", &relative], None)?;
        if file.exists() {
            std::fs::remove_file(file).map_err(AppError::from)?;
        }
    }

    let status = git.run_command(&["status", "--porcelain", "--", ".jules"], None)?;
    if !status.trim().is_empty() {
        let message = format!("jules: cleanup mock artifacts {}", mock_tag);
        git.run_command(&["add", "-u", ".jules"], None)?;
        git.run_command(&["commit", "-m", &message], None)?;

        let branch = git.get_current_branch()?;
        git.push_branch(&branch, false)?;
    }

    Ok(files.len())
}

fn collect_mock_files(jules_path: &Path, mock_tag: &str) -> Result<Vec<PathBuf>, AppError> {
    let mut stack = vec![jules_path.to_path_buf()];
    let mut files = Vec::new();

    while let Some(path) = stack.pop() {
        let metadata = std::fs::metadata(&path).map_err(AppError::from)?;
        if metadata.is_dir() {
            let entries = std::fs::read_dir(&path).map_err(AppError::from)?;
            for entry in entries {
                stack.push(entry.map_err(AppError::from)?.path());
            }
            continue;
        }

        if is_mock_file(&path, mock_tag)? {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn is_mock_file(path: &Path, mock_tag: &str) -> Result<bool, AppError> {
    let filename_contains_tag =
        path.file_name().is_some_and(|name| name.to_string_lossy().contains(mock_tag));
    if filename_contains_tag {
        return Ok(true);
    }

    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content.contains(mock_tag)),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => Ok(false),
        Err(error) => Err(AppError::from(error)),
    }
}

fn discover_mock_pr_numbers(repository: &str, mock_tag: &str) -> Result<Vec<u64>, AppError> {
    let output = run_command(
        "gh",
        &[
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "200",
            "--json",
            "number,headRefName,title",
            "--repo",
            repository,
        ],
    )?;
    parse_mock_pr_numbers(&output, mock_tag)
}

fn parse_mock_pr_numbers(json: &str, mock_tag: &str) -> Result<Vec<u64>, AppError> {
    let pull_requests: Vec<PullRequestSummary> = serde_json::from_str(json).map_err(|error| {
        AppError::ParseError { what: "gh pr list output".to_string(), details: error.to_string() }
    })?;

    let mut numbers: Vec<u64> = pull_requests
        .into_iter()
        .filter(|item| item.head_ref_name.contains(mock_tag) || item.title.contains(mock_tag))
        .map(|item| item.number)
        .collect();

    numbers.sort_unstable();
    numbers.dedup();
    Ok(numbers)
}

fn discover_mock_branches(
    git: &GitCommandAdapter,
    mock_tag: &str,
) -> Result<Vec<String>, AppError> {
    let output = git.run_command(&["ls-remote", "--heads", "origin"], None)?;
    Ok(parse_mock_branches(&output, mock_tag))
}

fn parse_mock_branches(ls_remote_output: &str, mock_tag: &str) -> Vec<String> {
    let mut branches: Vec<String> = ls_remote_output
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/heads/"))
        .filter(|branch| branch.contains(mock_tag))
        .map(str::to_string)
        .collect();

    branches.sort();
    branches.dedup();
    branches
}

fn run_command(program: &str, args: &[&str]) -> Result<String, AppError> {
    let output =
        Command::new(program).args(args).output().map_err(|error| AppError::ExternalToolError {
            tool: program.to_string(),
            error: format!("Failed to execute {}: {}", program, error),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::ExternalToolError {
            tool: program.to_string(),
            error: if stderr.is_empty() {
                format!("{} command failed with status {}", program, output.status)
            } else {
                stderr
            },
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn workspace_root(workspace: &FilesystemWorkspaceStore) -> Result<PathBuf, AppError> {
    let root = workspace.jules_path().parent().unwrap_or(Path::new(".")).to_path_buf();
    root.canonicalize().map_err(|error| {
        AppError::InternalError(format!("Failed to resolve workspace root: {}", error))
    })
}

fn to_repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string()
}

fn sort_and_dedup<T: Ord>(mut values: Vec<T>) -> Vec<T> {
    values.sort();
    values.dedup();
    values
}

#[derive(Debug, Deserialize)]
struct PullRequestSummary {
    number: u64,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    title: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn rejects_invalid_mock_tag() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".jules")).unwrap();
        fs::write(root.join(".jules/version"), env!("CARGO_PKG_VERSION")).unwrap();
        std::env::set_current_dir(root).unwrap();

        // Set required env vars for test
        unsafe {
            std::env::set_var("GH_TOKEN", "test");
            std::env::set_var("GITHUB_REPOSITORY", "owner/repo");
        }

        let result = execute(WorkflowCleanupMockOptions {
            mock_tag: "invalid-tag".to_string(),
            pr_numbers_json: None,
            branches_json: None,
        });

        unsafe {
            std::env::remove_var("GH_TOKEN");
            std::env::remove_var("GITHUB_REPOSITORY");
        }

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mock"));
    }

    #[test]
    fn parse_mock_pr_numbers_filters_by_tag() {
        let json = r#"
[
  {"number": 12, "headRefName": "jules-observer-mock-run-1", "title": "observer"},
  {"number": 13, "headRefName": "jules-observer-real", "title": "[mock-run-1] title match"},
  {"number": 14, "headRefName": "jules-observer-real", "title": "real"}
]
"#;

        let numbers = parse_mock_pr_numbers(json, "mock-run-1").unwrap();
        assert_eq!(numbers, vec![12, 13]);
    }

    #[test]
    fn parse_mock_branches_filters_by_tag() {
        let ls_remote = r#"
abc123	refs/heads/jules-decider-mock-run-1
def456	refs/heads/main
789abc	refs/heads/jules-implementer-bugs-a1b2c3-mock-run-1
"#;

        let branches = parse_mock_branches(ls_remote, "mock-run-1");
        assert_eq!(
            branches,
            vec![
                "jules-decider-mock-run-1".to_string(),
                "jules-implementer-bugs-a1b2c3-mock-run-1".to_string()
            ]
        );
    }
}
