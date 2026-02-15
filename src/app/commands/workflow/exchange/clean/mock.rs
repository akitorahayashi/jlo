//! Workflow cleanup mock command implementation.
//!
//! Closes mock PRs, deletes mock branches, removes mock-tagged files from jules branch.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde::Serialize;

use crate::adapters::git::GitCommandAdapter;
use crate::adapters::github::GitHubCommandAdapter;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::AppError;
use crate::ports::{Git, GitHub, JulesStore};

/// Options for workflow cleanup mock command.
#[derive(Debug, Clone)]
pub struct ExchangeCleanMockOptions {
    /// Mock tag to identify artifacts.
    pub mock_tag: String,
    /// PR numbers to close (optional, discovered if not provided).
    pub pr_numbers_json: Option<Vec<u64>>,
    /// Branches to delete (optional, discovered if not provided).
    pub branches_json: Option<Vec<String>>,
}

/// Output of workflow cleanup mock command.
#[derive(Debug, Clone, Serialize)]
pub struct ExchangeCleanMockOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Number of PRs closed.
    pub closed_prs_count: usize,
    /// Number of issues closed.
    pub closed_issues_count: usize,
    /// Number of branches deleted.
    pub deleted_branches_count: usize,
    /// Number of mock files deleted from jules branch.
    pub deleted_files_count: usize,
}

/// Execute cleanup mock command.
pub fn execute(options: ExchangeCleanMockOptions) -> Result<ExchangeCleanMockOutput, AppError> {
    let repository = LocalRepositoryAdapter::current()?;

    if !repository.jules_exists() {
        return Err(AppError::JulesNotFound);
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

    let github_repository = std::env::var("GITHUB_REPOSITORY").map_err(|_| {
        AppError::Validation("GITHUB_REPOSITORY environment variable is required".to_string())
    })?;
    let worker_branch = resolve_worker_branch(std::env::var("JULES_WORKER_BRANCH").ok())?;

    let root = repository_root(&repository)?;
    let git = GitCommandAdapter::new(root);
    let github = GitHubCommandAdapter::new();
    ensure_worker_branch_checked_out(&git, &worker_branch)?;

    let current_branch = git.get_current_branch()?;
    if current_branch != worker_branch {
        return Err(AppError::Validation(format!(
            "Mock cleanup must run on configured worker branch '{}', current branch is '{}'",
            worker_branch, current_branch
        )));
    }

    let pr_numbers = match options.pr_numbers_json {
        Some(values) => sort_and_dedup(values),
        None => discover_mock_pr_numbers(&github_repository, &options.mock_tag)?,
    };

    let branches = match options.branches_json {
        Some(values) => sort_and_dedup(values),
        None => discover_mock_branches(&git, &options.mock_tag)?,
    };
    let issue_numbers = discover_mock_issue_numbers(&github_repository, &options.mock_tag)?;

    let closed_prs_count = close_pull_requests(&github, &pr_numbers)?;
    let closed_issues_count = close_issues(&issue_numbers)?;
    let deleted_branches_count = delete_remote_branches(&github, &branches)?;
    let deleted_files_count =
        delete_mock_files(&repository, &git, &github, &options.mock_tag, &worker_branch)?;

    eprintln!(
        "Cleaned mock artifacts for tag '{}': {} PRs, {} issues, {} branches, {} files",
        options.mock_tag,
        closed_prs_count,
        closed_issues_count,
        deleted_branches_count,
        deleted_files_count
    );

    Ok(ExchangeCleanMockOutput {
        schema_version: 1,
        closed_prs_count,
        closed_issues_count,
        deleted_branches_count,
        deleted_files_count,
    })
}

fn ensure_worker_branch_checked_out(
    git: &GitCommandAdapter,
    worker_branch: &str,
) -> Result<(), AppError> {
    git.fetch("origin")?;
    git.run_command(
        &["checkout", "-B", worker_branch, &format!("origin/{}", worker_branch)],
        None,
    )?;
    Ok(())
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

fn close_issues(issue_numbers: &[u64]) -> Result<usize, AppError> {
    let mut closed_count = 0;
    for issue_number in issue_numbers {
        run_command(
            "gh",
            &["issue", "close", &issue_number.to_string(), "--comment", "Closing mock issue"],
        )?;
        closed_count += 1;
    }
    Ok(closed_count)
}

fn delete_mock_files(
    repository: &LocalRepositoryAdapter,
    git: &GitCommandAdapter,
    github: &GitHubCommandAdapter,
    mock_tag: &str,
    worker_branch: &str,
) -> Result<usize, AppError> {
    let root = repository_root(repository)?;
    let jules_path = repository.jules_path();

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
    if status.trim().is_empty() {
        return Ok(files.len());
    }

    let cleanup_branch = build_cleanup_branch_name(mock_tag);
    git.run_command(&["checkout", "-b", &cleanup_branch], None)?;

    let message = format!("jules: cleanup mock artifacts {}", mock_tag);
    git.run_command(&["add", "-u", ".jules"], None)?;
    git.run_command(&["commit", "-m", &message], None)?;
    git.push_branch(&cleanup_branch, false)?;

    let pr_title = format!("chore: cleanup mock artifacts {}", mock_tag);
    let pr_body = format!(
        "Automated cleanup for mock run `{}`.\n\n- remove mock-tagged runtime artifacts\n- close/delete related mock resources",
        mock_tag
    );
    // Cleanup uses a PR path to satisfy branch protection and preserve auditable merge history.
    // Auto-merge authority remains centralized in the dedicated `jules-automerge` workflow.
    github.create_pull_request(&cleanup_branch, worker_branch, &pr_title, &pr_body)?;

    Ok(files.len())
}

fn resolve_worker_branch(configured_worker_branch: Option<String>) -> Result<String, AppError> {
    let worker_branch = configured_worker_branch
        .ok_or_else(|| {
            AppError::Validation(
                "JULES_WORKER_BRANCH environment variable is required for cleanup mock".to_string(),
            )
        })?
        .trim()
        .to_string();

    if worker_branch.is_empty() {
        return Err(AppError::Validation(
            "JULES_WORKER_BRANCH environment variable must not be empty".to_string(),
        ));
    }

    Ok(worker_branch)
}

fn build_cleanup_branch_name(mock_tag: &str) -> String {
    let sanitized: String = mock_tag
        .chars()
        .map(
            |ch| {
                if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' { ch } else { '-' }
            },
        )
        .collect();
    format!("jules-mock-cleanup-{}", sanitized)
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

fn discover_mock_issue_numbers(repository: &str, mock_tag: &str) -> Result<Vec<u64>, AppError> {
    let output = run_command(
        "gh",
        &[
            "issue",
            "list",
            "--state",
            "open",
            "--limit",
            "200",
            "--json",
            "number,title,body",
            "--repo",
            repository,
        ],
    )?;
    parse_mock_issue_numbers(&output, mock_tag)
}

fn parse_mock_issue_numbers(json: &str, mock_tag: &str) -> Result<Vec<u64>, AppError> {
    let issues: Vec<IssueSummary> =
        serde_json::from_str(json).map_err(|error| AppError::ParseError {
            what: "gh issue list output".to_string(),
            details: error.to_string(),
        })?;

    let mut numbers: Vec<u64> = issues
        .into_iter()
        .filter(|item| {
            item.title.starts_with("[innovator/")
                && (item.title.contains(mock_tag)
                    || item.body.as_deref().is_some_and(|body| body.contains(mock_tag)))
        })
        .map(|item| item.number)
        .collect();

    numbers.sort_unstable();
    numbers.dedup();
    Ok(numbers)
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

fn repository_root(repository: &LocalRepositoryAdapter) -> Result<PathBuf, AppError> {
    let root = repository.jules_path().parent().unwrap_or(Path::new(".")).to_path_buf();
    root.canonicalize().map_err(|error| {
        AppError::InternalError(format!("Failed to resolve repository root: {}", error))
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

#[derive(Debug, Deserialize)]
struct IssueSummary {
    number: u64,
    title: String,
    #[serde(default)]
    body: Option<String>,
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

        let result = execute(ExchangeCleanMockOptions {
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

    #[test]
    fn parse_mock_issue_numbers_filters_innovator_issues_by_tag() {
        let json = r#"
[
  {"number": 21, "title": "[innovator/alice] Mock proposal", "body": "Mock tag: mock-run-1"},
  {"number": 22, "title": "[innovator/bob] Proposal", "body": "No mock tag"},
  {"number": 23, "title": "[decider] something", "body": "Mock tag: mock-run-1"},
  {"number": 24, "title": "[innovator/charlie] mock-run-1 from title", "body": ""}
]
"#;

        let numbers = parse_mock_issue_numbers(json, "mock-run-1").unwrap();
        assert_eq!(numbers, vec![21, 24]);
    }

    #[test]
    fn resolve_worker_branch_requires_non_empty_configured_value() {
        assert!(resolve_worker_branch(None).is_err());
        assert!(resolve_worker_branch(Some("".to_string())).is_err());
        assert_eq!(resolve_worker_branch(Some("jules".to_string())).unwrap(), "jules");
    }
}
