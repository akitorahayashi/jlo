use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::git_command::GitCommandAdapter;
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::domain::AppError;
use crate::ports::{GitPort, WorkspaceStore};

use super::inspect::{WorkflowWorkstreamsInspectOptions, inspect_at};

#[derive(Debug, Clone)]
pub struct WorkflowWorkstreamsCleanIssueOptions {
    pub issue_file: String,
}

#[derive(Debug, Serialize)]
pub struct WorkflowWorkstreamsCleanIssueOutput {
    pub schema_version: u32,
    pub deleted_paths: Vec<String>,
    pub committed: bool,
    pub commit_sha: String,
    pub pushed: bool,
}

pub fn execute(
    options: WorkflowWorkstreamsCleanIssueOptions,
) -> Result<WorkflowWorkstreamsCleanIssueOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = workspace.jules_path();
    let canonical_jules = jules_path
        .canonicalize()
        .map_err(|e| AppError::InternalError(format!("Failed to resolve .jules path: {}", e)))?;

    let issue_path = Path::new(&options.issue_file);
    let canonical_issue = issue_path.canonicalize().map_err(|_| {
        AppError::Validation(format!("Issue file does not exist: {}", options.issue_file))
    })?;

    if !canonical_issue.starts_with(&canonical_jules) {
        return Err(AppError::Validation(format!(
            "Issue file must be within .jules/ directory: {}",
            options.issue_file
        )));
    }

    let (workstream, issue_rel) =
        resolve_workstream_and_issue_path(&canonical_jules, &canonical_issue, &workspace)?;

    let canonical_root = canonical_jules.parent().unwrap_or(Path::new(".")).to_path_buf();
    let canonical_store = FilesystemWorkspaceStore::new(canonical_root);
    let inspect_output =
        inspect_at(&canonical_store, WorkflowWorkstreamsInspectOptions { workstream })?;

    let issue_item =
        inspect_output.issues.items.iter().find(|item| item.path == issue_rel).ok_or_else(
            || {
                AppError::Validation(format!(
                    "Issue file not found in inspection output: {}",
                    issue_rel
                ))
            },
        )?;

    let mut event_map: HashMap<&str, &str> = HashMap::new();
    for event in &inspect_output.events.items {
        event_map.insert(event.id.as_str(), event.path.as_str());
    }

    let mut deleted_paths = HashSet::new();
    for event_id in &issue_item.source_events {
        let event_path = event_map.get(event_id.as_str()).ok_or_else(|| {
            AppError::Validation(format!(
                "Source event '{}' not found in inspection output",
                event_id
            ))
        })?;
        deleted_paths.insert(event_path.to_string());
    }

    deleted_paths.insert(issue_rel.clone());

    let mut deleted_paths: Vec<String> = deleted_paths.into_iter().collect();
    deleted_paths.sort();

    if deleted_paths.is_empty() {
        return Err(AppError::Validation(
            "No files resolved for cleanup; aborting to avoid empty commit".to_string(),
        ));
    }

    let root = workspace_root(&workspace)?;
    let git = GitCommandAdapter::new(root);

    for path in &deleted_paths {
        git.run_command(&["rm", "--", path], None)?;
    }

    let commit_message = format!("jules: clean issue {}", issue_item.id);
    git.run_command(&["commit", "-m", &commit_message], None)?;
    let commit_sha = git.get_head_sha()?;

    let branch = git.get_current_branch()?;
    if branch.trim().is_empty() {
        return Err(AppError::Validation(
            "Cannot push cleanup commit: current branch name is empty".to_string(),
        ));
    }

    git.push_branch(&branch, false)?;

    Ok(WorkflowWorkstreamsCleanIssueOutput {
        schema_version: 1,
        deleted_paths,
        committed: true,
        commit_sha,
        pushed: true,
    })
}

fn resolve_workstream_and_issue_path(
    canonical_jules: &Path,
    canonical_issue: &Path,
    workspace: &FilesystemWorkspaceStore,
) -> Result<(String, String), AppError> {
    let rel_to_jules = canonical_issue
        .strip_prefix(canonical_jules)
        .map_err(|_| AppError::Validation("Issue file is not under .jules/".to_string()))?;

    let parts: Vec<String> =
        rel_to_jules.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();

    if parts.len() < 6
        || parts[0] != "workstreams"
        || parts[2] != "exchange"
        || parts[3] != "issues"
    {
        return Err(AppError::Validation(format!(
            "Issue file must be under .jules/workstreams/<name>/exchange/issues/: {}",
            canonical_issue.display()
        )));
    }

    let workstream = parts[1].clone();
    let root = workspace_root(workspace)?;
    let issue_rel = to_repo_relative(&root, canonical_issue);

    Ok((workstream, issue_rel))
}

fn workspace_root(workspace: &FilesystemWorkspaceStore) -> Result<PathBuf, AppError> {
    let root = workspace.jules_path().parent().unwrap_or(Path::new(".")).to_path_buf();
    root.canonicalize()
        .map_err(|e| AppError::InternalError(format!("Failed to resolve workspace root: {}", e)))
}

fn to_repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn clean_issue_deletes_files_and_pushes() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let repo_dir = root.join("repo");
        let remote_dir = root.join("remote.git");
        fs::create_dir_all(&repo_dir).unwrap();

        Command::new("git").args(["init"]).current_dir(&repo_dir).output().unwrap();
        Command::new("git")
            .args(["checkout", "-b", "jules"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();

        Command::new("git")
            .args(["init", "--bare", remote_dir.to_str().unwrap()])
            .output()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", remote_dir.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output()
            .unwrap();

        let jules_path = repo_dir.join(".jules");
        let ws_dir = jules_path.join("workstreams/alpha/exchange");
        fs::create_dir_all(ws_dir.join("events/pending")).unwrap();
        fs::create_dir_all(ws_dir.join("issues/bugs")).unwrap();

        fs::write(ws_dir.join("events/pending/event1.yml"), "id: abc123\n").unwrap();
        fs::write(ws_dir.join("events/pending/event2.yml"), "id: def456\n").unwrap();
        fs::write(
            ws_dir.join("issues/bugs/issue.yml"),
            r#"
id: abc123
source_events:
  - abc123
  - def456
requires_deep_analysis: false
"#,
        )
        .unwrap();

        fs::write(
            jules_path.join("workstreams/alpha/scheduled.toml"),
            r#"
version = 1
enabled = true
[observers]
roles = [
    { name = "taxonomy", enabled = true },
]
[deciders]
roles = []
"#,
        )
        .unwrap();

        Command::new("git").args(["add", ".jules"]).current_dir(&repo_dir).output().unwrap();
        Command::new("git").args(["commit", "-m", "seed"]).current_dir(&repo_dir).output().unwrap();

        std::env::set_current_dir(&repo_dir).unwrap();

        let output = execute(WorkflowWorkstreamsCleanIssueOptions {
            issue_file: ".jules/workstreams/alpha/exchange/issues/bugs/issue.yml".to_string(),
        })
        .unwrap();

        assert_eq!(output.schema_version, 1);
        assert!(output.deleted_paths.iter().any(|p| p.contains("event1.yml")));
        assert!(output.deleted_paths.iter().any(|p| p.contains("issue.yml")));

        assert!(
            !repo_dir.join(".jules/workstreams/alpha/exchange/events/pending/event1.yml").exists()
        );
        assert!(!repo_dir.join(".jules/workstreams/alpha/exchange/issues/bugs/issue.yml").exists());

        let head = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        let head_sha = String::from_utf8_lossy(&head.stdout).trim().to_string();

        let remote_head = Command::new("git")
            .args(["ls-remote", "origin", "refs/heads/jules"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        let remote_line = String::from_utf8_lossy(&remote_head.stdout);
        let remote_sha = remote_line.split_whitespace().next().unwrap_or("");

        assert_eq!(head_sha, remote_sha);
    }
}
