use crate::domain::AppError;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct DiffStat {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub subject: String,
}

pub trait GitPort {
    /// Get the current HEAD SHA.
    fn get_head_sha(&self) -> Result<String, AppError>;

    /// Get the current branch name.
    fn get_current_branch(&self) -> Result<String, AppError>;

    /// Get the URL for a remote.
    fn get_remote_url(&self, name: &str) -> Result<String, AppError>;

    /// Check if a commit exists.
    fn commit_exists(&self, sha: &str) -> bool;

    /// Get the Nth ancestor of a commit.
    fn get_nth_ancestor(&self, commit: &str, n: usize) -> Result<String, AppError>;

    /// Check if there are changes in the range matching the pathspec.
    fn has_changes(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<bool, AppError>;

    /// Count commits in the range matching the pathspec.
    fn count_commits(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<u32, AppError>;

    /// Collect commits in the range matching the pathspec.
    fn collect_commits(
        &self,
        from: &str,
        to: &str,
        pathspec: &[&str],
        limit: usize,
    ) -> Result<Vec<CommitInfo>, AppError>;

    /// Collect diffstat for the range matching the pathspec.
    fn get_diffstat(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<DiffStat, AppError>;

    /// Execute an arbitrary git command (fallback).
    #[allow(dead_code)]
    fn run_command(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, AppError>;

    // === Mock mode operations ===

    /// Checkout a branch, optionally creating it.
    fn checkout_branch(&self, branch: &str, create: bool) -> Result<(), AppError>;

    /// Push a branch to the remote.
    fn push_branch(&self, branch: &str, force: bool) -> Result<(), AppError>;

    /// Stage and commit files with a message.
    fn commit_files(&self, message: &str, files: &[&Path]) -> Result<String, AppError>;

    /// Fetch from remote.
    fn fetch(&self, remote: &str) -> Result<(), AppError>;

    /// Delete a local branch. Returns true if the branch was deleted.
    fn delete_branch(&self, branch: &str, force: bool) -> Result<bool, AppError>;
}
