use crate::domain::AppError;
use crate::ports::{CommitInfo, DiffStat, GitPort};
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

impl GitPort for GitCommandAdapter {
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

    fn count_commits(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<u32, AppError> {
        let range = format!("{}..{}", from, to);
        let mut args = vec!["rev-list", "--count", &range, "--"];
        args.extend(pathspec);

        let output = self.run(&args, None)?;
        output.trim().parse().map_err(|e| AppError::ParseError {
            what: "commit count".to_string(),
            details: format!("Value: '{}', Error: {}", output, e),
        })
    }

    fn collect_commits(
        &self,
        from: &str,
        to: &str,
        pathspec: &[&str],
        limit: usize,
    ) -> Result<Vec<CommitInfo>, AppError> {
        let range = format!("{}..{}", from, to);
        let limit_arg = format!("-{}", limit);
        let mut args = vec!["log", &limit_arg, "--pretty=format:%H|%s", &range, "--"];
        args.extend(pathspec);

        let output = self.run(&args, None)?;

        let mut commits = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                commits
                    .push(CommitInfo { sha: parts[0].to_string(), subject: parts[1].to_string() });
            }
        }

        Ok(commits)
    }

    fn get_diffstat(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<DiffStat, AppError> {
        let range = format!("{}..{}", from, to);
        let mut args = vec!["diff", "--numstat", &range, "--"];
        args.extend(pathspec);

        let output = self.run(&args, None)?;
        let mut stat = DiffStat::default();

        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                if parts[0] != "-" {
                    stat.insertions += parts[0].parse::<u32>().unwrap_or(0);
                }
                if parts[1] != "-" {
                    stat.deletions += parts[1].parse::<u32>().unwrap_or(0);
                }
                stat.files_changed += 1;
            }
        }

        Ok(stat)
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
                AppError::Validation { reason: "File path contains invalid unicode".to_string() }
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
