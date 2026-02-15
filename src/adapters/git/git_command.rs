use crate::domain::AppError;
use crate::ports::Git;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitCommandAdapter {
    root: PathBuf,
}

impl GitCommandAdapter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn run(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, AppError> {
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
        self.run(&["cat-file", "-e", sha], None).is_ok()
    }

    fn get_nth_ancestor(&self, commit: &str, n: usize) -> Result<String, AppError> {
        match self.run(&["rev-parse", &format!("{}~{}", commit, n)], None) {
            Ok(sha) => Ok(sha),
            Err(_) => {
                // Fallback: first commit in ancestry
                let first = self.run(&["rev-list", "--max-parents=0", commit], None)?;
                Ok(first.lines().next().unwrap_or("").to_string())
            }
        }
    }

    fn has_changes(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<bool, AppError> {
        let range = format!("{}..{}", from, to);
        let mut args = vec!["diff", "--name-only", &range, "--"];
        args.extend(pathspec);
        let output = self.run(&args, None)?;
        Ok(!output.trim().is_empty())
    }

    fn checkout_branch(&self, branch: &str, create: bool) -> Result<(), AppError> {
        let args = if create { vec!["checkout", "-b", branch] } else { vec!["checkout", branch] };
        self.run(&args, None)?;
        Ok(())
    }

    fn push_branch(&self, branch: &str, force: bool) -> Result<(), AppError> {
        let args = if force {
            vec!["push", "-f", "-u", "origin", branch]
        } else {
            vec!["push", "-u", "origin", branch]
        };
        self.run(&args, None)?;
        Ok(())
    }

    fn commit_files(&self, message: &str, files: &[&Path]) -> Result<String, AppError> {
        // Stage files
        for file in files {
            let path_str = file.to_str().ok_or_else(|| {
                AppError::Validation("File path contains invalid unicode".to_string())
            })?;
            self.run(&["add", path_str], None)?;
        }

        // Commit
        self.run(&["commit", "-m", message], None)?;

        // Return new HEAD SHA
        self.get_head_sha()
    }

    fn fetch(&self, remote: &str) -> Result<(), AppError> {
        self.run(&["fetch", remote], None)?;
        Ok(())
    }

    fn delete_branch(&self, branch: &str, force: bool) -> Result<bool, AppError> {
        let output = self.run(&["branch", "--list", branch], None)?;
        if output.trim().is_empty() {
            return Ok(false);
        }

        let args = if force { vec!["branch", "-D", branch] } else { vec!["branch", "-d", branch] };
        self.run(&args, None)?;
        Ok(true)
    }
}
