use crate::domain::AppError;
use crate::ports::Git;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
        Ok(String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").to_string())
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
}
