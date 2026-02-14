use crate::domain::AppError;
use crate::ports::GitPort;
use git2::{DiffOptions, Oid, Repository};
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

    fn repo(&self) -> Result<Repository, AppError> {
        Repository::open(&self.root).map_err(|e| AppError::GitError {
            command: "git2::Repository::open".to_string(),
            details: e.to_string(),
        })
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
        let repo = self.repo()?;
        let head = repo.head().map_err(|e| AppError::GitError {
            command: "git2::Repository::head".to_string(),
            details: e.to_string(),
        })?;
        let target = head.target().ok_or_else(|| AppError::GitError {
            command: "git2::Reference::target".to_string(),
            details: "HEAD is not a direct reference".to_string(),
        })?;
        Ok(target.to_string())
    }

    fn get_current_branch(&self) -> Result<String, AppError> {
        let repo = self.repo()?;

        match repo.head() {
            Ok(head) => {
                let shorthand = head.shorthand().ok_or_else(|| AppError::GitError {
                    command: "git2::Reference::shorthand".to_string(),
                    details: "HEAD has no shorthand".to_string(),
                })?;
                Ok(shorthand.to_string())
            }
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
                let head_ref = repo.find_reference("HEAD").map_err(|e| AppError::GitError {
                    command: "git2::Repository::find_reference(HEAD)".to_string(),
                    details: e.to_string(),
                })?;

                if let Some(target) = head_ref.symbolic_target() {
                    Ok(target.strip_prefix("refs/heads/").unwrap_or(target).to_string())
                } else {
                    Err(AppError::GitError {
                        command: "get_current_branch".to_string(),
                        details: "HEAD is detached and unborn".to_string(),
                    })
                }
            }
            Err(e) => Err(AppError::GitError {
                command: "git2::Repository::head".to_string(),
                details: e.to_string(),
            }),
        }
    }

    fn commit_exists(&self, sha: &str) -> bool {
        let repo = match self.repo() {
            Ok(r) => r,
            Err(_) => return false,
        };
        match Oid::from_str(sha) {
            Ok(oid) => repo.find_commit(oid).is_ok(),
            Err(_) => false,
        }
    }

    fn get_nth_ancestor(&self, commit: &str, n: usize) -> Result<String, AppError> {
        let repo = self.repo()?;
        let oid = Oid::from_str(commit).map_err(|e| AppError::GitError {
            command: "git2::Oid::from_str".to_string(),
            details: e.to_string(),
        })?;
        let commit_obj = repo.find_commit(oid).map_err(|e| AppError::GitError {
            command: "git2::Repository::find_commit".to_string(),
            details: e.to_string(),
        })?;

        let mut current = commit_obj;
        for _ in 0..n {
            if current.parent_count() == 0 {
                return Ok(current.id().to_string());
            }
            match current.parent(0) {
                Ok(p) => current = p,
                Err(_) => break,
            }
        }
        Ok(current.id().to_string())
    }

    fn has_changes(&self, from: &str, to: &str, pathspec: &[&str]) -> Result<bool, AppError> {
        let repo = self.repo()?;
        let from_oid = Oid::from_str(from).map_err(|e| AppError::GitError {
            command: "git2::Oid::from_str".to_string(),
            details: e.to_string(),
        })?;
        let to_oid = Oid::from_str(to).map_err(|e| AppError::GitError {
            command: "git2::Oid::from_str".to_string(),
            details: e.to_string(),
        })?;

        let from_tree =
            repo.find_commit(from_oid).and_then(|c| c.tree()).map_err(|e| AppError::GitError {
                command: "git2::Repository::find_commit/tree".to_string(),
                details: e.to_string(),
            })?;
        let to_tree =
            repo.find_commit(to_oid).and_then(|c| c.tree()).map_err(|e| AppError::GitError {
                command: "git2::Repository::find_commit/tree".to_string(),
                details: e.to_string(),
            })?;

        let mut opts = DiffOptions::new();
        for p in pathspec {
            opts.pathspec(p);
        }

        let diff = repo
            .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut opts))
            .map_err(|e| AppError::GitError {
                command: "git2::Repository::diff_tree_to_tree".to_string(),
                details: e.to_string(),
            })?;

        Ok(diff.deltas().len() > 0)
    }

    fn checkout_branch(&self, branch: &str, create: bool) -> Result<(), AppError> {
        let repo = self.repo()?;

        if create {
            let head_commit =
                repo.head().and_then(|h| h.peel_to_commit()).map_err(|e| AppError::GitError {
                    command: "git2::Repository::head".to_string(),
                    details: e.to_string(),
                })?;
            repo.branch(branch, &head_commit, false).map_err(|e| AppError::GitError {
                command: "git2::Repository::branch".to_string(),
                details: e.to_string(),
            })?;
        }

        let refname = format!("refs/heads/{}", branch);

        // Use checkout_tree before set_head to ensure safety
        let obj = repo.find_reference(&refname).and_then(|r| r.peel_to_commit()).map_err(|e| {
            AppError::GitError {
                command: "git2::Repository::find_reference".to_string(),
                details: e.to_string(),
            }
        })?;

        let mut builder = git2::build::CheckoutBuilder::new();
        // Safe checkout is default
        repo.checkout_tree(obj.as_object(), Some(&mut builder)).map_err(|e| {
            AppError::GitError {
                command: "git2::Repository::checkout_tree".to_string(),
                details: e.to_string(),
            }
        })?;

        repo.set_head(&refname).map_err(|e| AppError::GitError {
            command: "git2::Repository::set_head".to_string(),
            details: e.to_string(),
        })?;

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
        let repo = self.repo()?;
        let mut index = repo.index().map_err(|e| AppError::GitError {
            command: "git2::Repository::index".to_string(),
            details: e.to_string(),
        })?;

        for file in files {
            let rel_path = if file.is_absolute() {
                file.strip_prefix(&self.root).map_err(|_| {
                    AppError::Validation(format!(
                        "File {:?} is not inside repository root {:?}",
                        file, self.root
                    ))
                })?
            } else {
                file
            };

            index.add_path(rel_path).map_err(|e| AppError::GitError {
                command: format!("git2::Index::add_path {:?}", rel_path),
                details: e.to_string(),
            })?;
        }
        index.write().map_err(|e| AppError::GitError {
            command: "git2::Index::write".to_string(),
            details: e.to_string(),
        })?;

        let tree_id = index.write_tree().map_err(|e| AppError::GitError {
            command: "git2::Index::write_tree".to_string(),
            details: e.to_string(),
        })?;
        let tree = repo.find_tree(tree_id).map_err(|e| AppError::GitError {
            command: "git2::Repository::find_tree".to_string(),
            details: e.to_string(),
        })?;

        let signature = repo.signature().map_err(|e| AppError::GitError {
            command: "git2::Repository::signature".to_string(),
            details: e.to_string(),
        })?;

        let parents = match repo.head() {
            Ok(head) => {
                let commit = head.peel_to_commit().map_err(|e| AppError::GitError {
                    command: "git2::Reference::peel_to_commit".to_string(),
                    details: e.to_string(),
                })?;
                vec![commit]
            }
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => vec![],
            Err(e) => {
                return Err(AppError::GitError {
                    command: "git2::Repository::head".to_string(),
                    details: e.to_string(),
                });
            }
        };

        let parents_refs: Vec<&git2::Commit> = parents.iter().collect();

        let oid = repo
            .commit(Some("HEAD"), &signature, &signature, message, &tree, &parents_refs)
            .map_err(|e| AppError::GitError {
                command: "git2::Repository::commit".to_string(),
                details: e.to_string(),
            })?;

        Ok(oid.to_string())
    }

    fn fetch(&self, remote: &str) -> Result<(), AppError> {
        self.run(&["fetch", remote], None)?;
        Ok(())
    }

    fn delete_branch(&self, branch: &str, force: bool) -> Result<bool, AppError> {
        let repo = self.repo()?;
        let mut branch_ref = match repo.find_branch(branch, git2::BranchType::Local) {
            Ok(b) => b,
            Err(_) => return Ok(false),
        };

        if !force {
            let branch_oid = branch_ref.get().target().ok_or_else(|| AppError::GitError {
                command: "delete_branch".to_string(),
                details: "Branch ref has no target".to_string(),
            })?;

            let head_oid = match repo.head() {
                Ok(h) => match h.target() {
                    Some(target) => target,
                    None => {
                        return Err(AppError::GitError {
                            command: "delete_branch".to_string(),
                            details: "HEAD has no target".to_string(),
                        });
                    }
                },
                Err(_) => {
                    return Err(AppError::GitError {
                        command: "delete_branch".to_string(),
                        details: "Cannot delete branch safely: HEAD not found".to_string(),
                    });
                }
            };

            if !repo.graph_descendant_of(head_oid, branch_oid).unwrap_or(false) {
                return Err(AppError::GitError {
                    command: "delete_branch".to_string(),
                    details: "Branch is not fully merged (use force to delete)".to_string(),
                });
            }
        }

        branch_ref.delete().map_err(|e| AppError::GitError {
            command: "git2::Branch::delete".to_string(),
            details: e.to_string(),
        })?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, GitCommandAdapter) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Initialize git repo
        let output = Command::new("git")
            .arg("init")
            .current_dir(&root)
            .output()
            .expect("Failed to init git repo");
        assert!(output.status.success());

        // Configure user for commits
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&root)
            .output()
            .unwrap();
        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&root)
            .output()
            .unwrap();

        // Create an initial commit so HEAD exists
        fs::write(root.join("README.md"), "# Test").unwrap();
        Command::new("git").args(&["add", "."]).current_dir(&root).output().unwrap();
        Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(&root)
            .output()
            .unwrap();

        (temp_dir, GitCommandAdapter::new(root))
    }

    #[test]
    fn test_get_head_sha() {
        let (_dir, git) = setup_repo();
        let sha = git.get_head_sha().expect("Failed to get HEAD SHA");
        assert_eq!(sha.len(), 40);
    }

    #[test]
    fn test_get_current_branch() {
        let (_dir, git) = setup_repo();
        let branch = git.get_current_branch().expect("Failed to get branch");
        // default branch is usually master or main depending on config, but we can verify it's not empty
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_get_current_branch_empty_repo() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        Command::new("git").arg("init").current_dir(&root).output().unwrap();

        let git = GitCommandAdapter::new(root);
        let branch = git.get_current_branch().expect("Failed to get branch in empty repo");
        assert!(!branch.is_empty()); // usually "master" or "main"
    }

    #[test]
    fn test_initial_commit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        Command::new("git").arg("init").current_dir(&root).output().unwrap();
        // Configure user for commits
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&root)
            .output()
            .unwrap();
        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&root)
            .output()
            .unwrap();

        let git = GitCommandAdapter::new(root.clone());
        let file_path = root.join("new_file.txt");
        fs::write(&file_path, "content").unwrap();

        // This should not panic or error now
        git.commit_files("Initial commit", &[&file_path]).expect("Failed to create initial commit");
        assert!(git.commit_exists(&git.get_head_sha().unwrap()));
    }

    #[test]
    fn test_commit_exists() {
        let (_dir, git) = setup_repo();
        let sha = git.get_head_sha().unwrap();
        assert!(git.commit_exists(&sha));
        assert!(!git.commit_exists("0000000000000000000000000000000000000000"));
    }

    #[test]
    fn test_checkout_branch() {
        let (_dir, git) = setup_repo();
        git.checkout_branch("new-branch", true).expect("Failed to create branch");
        let current = git.get_current_branch().unwrap();
        assert_eq!(current, "new-branch");

        git.checkout_branch("master", false)
            .or_else(|_| git.checkout_branch("main", false))
            .expect("Failed to checkout original branch");
    }

    #[test]
    fn test_commit_files() {
        let (dir, git) = setup_repo();
        let file_path = dir.path().join("new_file.txt");
        fs::write(&file_path, "content").unwrap();

        let sha_before = git.get_head_sha().unwrap();
        let sha_after = git.commit_files("Add file", &[&file_path]).expect("Failed to commit");

        assert_ne!(sha_before, sha_after);
        assert!(git.commit_exists(&sha_after));
    }

    #[test]
    fn test_has_changes() {
        let (dir, git) = setup_repo();
        // Create a new commit
        let file_path = dir.path().join("file2.txt");
        fs::write(&file_path, "content").unwrap();
        git.commit_files("Commit 2", &[&file_path]).unwrap();

        let head = git.get_head_sha().unwrap();
        let parent = git.get_nth_ancestor(&head, 1).unwrap();

        // Check diff between parent and head
        assert!(git.has_changes(&parent, &head, &["file2.txt"]).unwrap());
        assert!(!git.has_changes(&parent, &head, &["README.md"]).unwrap());
    }

    #[test]
    fn test_get_nth_ancestor() {
        let (dir, git) = setup_repo();
        // Initial commit is 0. Make more commits.
        for i in 1..=3 {
            let file_path = dir.path().join(format!("file{}.txt", i));
            fs::write(&file_path, "content").unwrap();
            git.commit_files(&format!("Commit {}", i), &[&file_path]).unwrap();
        }

        let head = git.get_head_sha().unwrap();
        let ancestor_1 = git.get_nth_ancestor(&head, 1).unwrap();
        let ancestor_2 = git.get_nth_ancestor(&head, 2).unwrap();

        assert_ne!(head, ancestor_1);
        assert_ne!(ancestor_1, ancestor_2);

        // ancestor_1 should be parent of head
        let parent_check = git.run(&["rev-parse", &format!("{}~1", head)], None).unwrap();
        assert_eq!(ancestor_1, parent_check);
    }

    #[test]
    fn test_delete_branch() {
        let (_dir, git) = setup_repo();
        git.checkout_branch("to-delete", true).unwrap();

        // Checkout master/main again so we can delete the other branch
        git.checkout_branch("master", false)
            .or_else(|_| git.checkout_branch("main", false))
            .unwrap();

        let deleted = git.delete_branch("to-delete", true).expect("Failed to delete branch");
        assert!(deleted);

        // Check it's gone
        let branches = git.run(&["branch"], None).unwrap();
        assert!(!branches.contains("to-delete"));
    }
}
