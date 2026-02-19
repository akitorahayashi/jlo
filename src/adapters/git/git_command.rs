use crate::domain::jlo_paths;
use crate::domain::{AppError, IoErrorKind};
use crate::ports::{Git, GitWorkspace};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct GitCommandAdapter {
    root: PathBuf,
}

impl GitCommandAdapter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn run_output(&self, args: &[&str], cwd: Option<&Path>) -> Result<Output, AppError> {
        let mut command = Command::new("git");
        command.args(args);
        command.current_dir(cwd.unwrap_or(&self.root));

        let output = command.output().map_err(|e| AppError::GitError {
            command: format!("git {}", args.join(" ")),
            details: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(AppError::GitError {
                command: format!("git {}", args.join(" ")),
                details: if stderr.is_empty() { "Unknown error".to_string() } else { stderr },
            });
        }

        Ok(output)
    }

    fn run(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, AppError> {
        let output = self.run_output(args, cwd)?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl Git for GitCommandAdapter {
    fn run_command(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, AppError> {
        self.run(args, cwd)
    }

    fn get_head_sha(&self) -> Result<String, AppError> {
        self.run(&["rev-parse", "HEAD"], None)
    }

    fn get_current_branch(&self) -> Result<String, AppError> {
        self.run(&["branch", "--show-current"], None)
    }

    fn commit_exists(&self, sha: &str) -> bool {
        self.run_output(&["cat-file", "-e", sha], None).is_ok()
    }

    fn get_nth_ancestor(&self, commit: &str, n: usize) -> Result<Option<String>, AppError> {
        if !self.commit_exists(commit) {
            return Err(AppError::GitError {
                command: format!("git rev-parse {}~{}", commit, n),
                details: format!("Commit {} does not exist", commit),
            });
        }

        match self.run_output(&["rev-parse", &format!("{}~{}", commit, n)], None) {
            Ok(output) => Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string())),
            Err(_) => Ok(None),
        }
    }

    fn get_first_commit(&self, commit: &str) -> Result<String, AppError> {
        let output = self.run_output(&["rev-list", "--max-parents=0", commit], None)?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::GitError {
                command: format!("git rev-list --max-parents=0 {}", commit),
                details: "Could not find first commit in ancestry.".to_string(),
            })
    }

    fn has_changes(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<bool, AppError> {
        let range = format!("{}..{}", from, to);
        let mut args = vec!["diff", "--name-only", &range, "--"];
        args.extend(pathspec);
        let output = self.run_output(&args, None)?;
        Ok(!output.stdout.is_empty())
    }

    fn checkout_branch(&self, branch: &str, create: bool) -> Result<(), AppError> {
        let args = if create { vec!["checkout", "-b", branch] } else { vec!["checkout", branch] };
        self.run_output(&args, None)?;
        Ok(())
    }

    fn push_branch_from_rev(&self, rev: &str, branch: &str, force: bool) -> Result<(), AppError> {
        let refspec = format!("{}:refs/heads/{}", rev, branch);
        let args = if force {
            vec!["push", "-f", "origin", &refspec]
        } else {
            vec!["push", "origin", &refspec]
        };
        self.run_output(&args, None)?;
        Ok(())
    }

    fn push_branch(&self, branch: &str, force: bool) -> Result<(), AppError> {
        let args = if force {
            vec!["push", "-f", "-u", "origin", branch]
        } else {
            vec!["push", "-u", "origin", branch]
        };
        self.run_output(&args, None)?;
        Ok(())
    }

    fn commit_files(&self, message: &str, files: &[&Path]) -> Result<String, AppError> {
        // Stage files
        for file in files {
            let path_str = file.to_str().ok_or_else(|| {
                AppError::Validation("File path contains invalid unicode".to_string())
            })?;
            self.run_output(&["add", path_str], None)?;
        }

        // Commit
        self.run_output(&["commit", "-m", message], None)?;

        // Return new HEAD SHA
        self.get_head_sha()
    }

    fn fetch(&self, remote: &str) -> Result<(), AppError> {
        self.run_output(&["fetch", remote], None)?;
        Ok(())
    }

    fn delete_branch(&self, branch: &str, force: bool) -> Result<bool, AppError> {
        let output = self.run_output(&["branch", "--list", branch], None)?;
        if output.stdout.is_empty() {
            return Ok(false);
        }

        let args = if force { vec!["branch", "-D", branch] } else { vec!["branch", "-d", branch] };
        self.run_output(&args, None)?;
        Ok(true)
    }

    fn create_workspace(&self, branch: &str) -> Result<Box<dyn GitWorkspace>, AppError> {
        let workspaces_dir = jlo_paths::workspaces_dir(&self.root);
        std::fs::create_dir_all(&workspaces_dir).map_err(|e| AppError::Io {
            message: format!("Failed to create workspaces directory: {}", e),
            kind: e.kind().into(),
        })?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let id = format!("ws-{}-{}", std::process::id(), now);
        let temp_dir = workspaces_dir.join(&id);

        let temp_dir_str = temp_dir.to_str().ok_or_else(|| AppError::Io {
            message: "Temporary workspace path is not valid UTF-8".to_string(),
            kind: IoErrorKind::Other,
        })?;

        // git worktree add --detach <path> <branch>
        // We use --detach to allow creating a workspace even if the branch is already checked out elsewhere.
        self.run_output(&["worktree", "add", "--detach", temp_dir_str, branch], None)?;

        Ok(Box::new(GitWorktreeWorkspace {
            adapter: GitCommandAdapter::new(temp_dir.clone()),
            temp_dir,
            main_root: self.root.clone(),
        }))
    }
}

struct GitWorktreeWorkspace {
    adapter: GitCommandAdapter,
    temp_dir: PathBuf,
    main_root: PathBuf,
}

impl Git for GitWorktreeWorkspace {
    fn get_head_sha(&self) -> Result<String, AppError> {
        self.adapter.get_head_sha()
    }

    fn get_current_branch(&self) -> Result<String, AppError> {
        self.adapter.get_current_branch()
    }

    fn commit_exists(&self, sha: &str) -> bool {
        self.adapter.commit_exists(sha)
    }

    fn get_nth_ancestor(&self, commit: &str, n: usize) -> Result<Option<String>, AppError> {
        self.adapter.get_nth_ancestor(commit, n)
    }

    fn get_first_commit(&self, commit: &str) -> Result<String, AppError> {
        self.adapter.get_first_commit(commit)
    }

    fn has_changes(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<bool, AppError> {
        self.adapter.has_changes(from, to, pathspec)
    }

    fn run_command(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, AppError> {
        self.adapter.run_command(args, cwd)
    }

    fn checkout_branch(&self, branch: &str, create: bool) -> Result<(), AppError> {
        self.adapter.checkout_branch(branch, create)
    }

    fn push_branch(&self, branch: &str, force: bool) -> Result<(), AppError> {
        self.adapter.push_branch(branch, force)
    }

    fn push_branch_from_rev(&self, rev: &str, branch: &str, force: bool) -> Result<(), AppError> {
        self.adapter.push_branch_from_rev(rev, branch, force)
    }

    fn commit_files(&self, message: &str, files: &[&Path]) -> Result<String, AppError> {
        self.adapter.commit_files(message, files)
    }

    fn fetch(&self, remote: &str) -> Result<(), AppError> {
        self.adapter.fetch(remote)
    }

    fn delete_branch(&self, branch: &str, force: bool) -> Result<bool, AppError> {
        self.adapter.delete_branch(branch, force)
    }

    fn create_workspace(&self, branch: &str) -> Result<Box<dyn GitWorkspace>, AppError> {
        self.adapter.create_workspace(branch)
    }
}

impl GitWorkspace for GitWorktreeWorkspace {
    fn path(&self) -> &Path {
        &self.temp_dir
    }
}

impl Drop for GitWorktreeWorkspace {
    fn drop(&mut self) {
        // try to remove worktree; pass the path directly to avoid a panic on non-UTF-8 paths
        let _ = Command::new("git")
            .arg("worktree")
            .arg("remove")
            .arg("-f")
            .arg(&self.temp_dir)
            .current_dir(&self.main_root)
            .output();
    }
}
