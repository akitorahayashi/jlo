use crate::domain::AppError;
use std::path::Path;

pub trait GitPort {
    /// Get the current HEAD SHA.
    fn get_head_sha(&self) -> Result<String, AppError>;

    /// Get the current branch name.
    fn get_current_branch(&self) -> Result<String, AppError>;

    /// Check if a commit exists.
    fn commit_exists(&self, sha: &str) -> bool;

    /// Get the Nth ancestor of a commit.
    fn get_nth_ancestor(&self, commit: &str, n: usize) -> Result<String, AppError>;

    /// Check if there are changes in the range matching the pathspec.
    fn has_changes(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<bool, AppError>;

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
