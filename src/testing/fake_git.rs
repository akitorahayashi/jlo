use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::domain::AppError;
use crate::ports::GitPort;

#[derive(Default)]
pub struct FakeGit {
    pub committed_files: Mutex<Vec<PathBuf>>,
    pub branches_created: Mutex<Vec<String>>,
    pub head_sha: Mutex<String>,
    pub current_branch: Mutex<String>,
}

impl FakeGit {
    pub fn new() -> Self {
        Self {
            committed_files: Mutex::new(Vec::new()),
            branches_created: Mutex::new(Vec::new()),
            head_sha: Mutex::new("abc123".to_string()),
            current_branch: Mutex::new("jules".to_string()),
        }
    }

    pub fn set_head_sha(&self, sha: &str) {
        *self.head_sha.lock().unwrap() = sha.to_string();
    }

    pub fn set_current_branch(&self, branch: &str) {
        *self.current_branch.lock().unwrap() = branch.to_string();
    }
}

impl GitPort for FakeGit {
    fn get_head_sha(&self) -> Result<String, AppError> {
        Ok(self.head_sha.lock().unwrap().clone())
    }

    fn get_current_branch(&self) -> Result<String, AppError> {
        Ok(self.current_branch.lock().unwrap().clone())
    }

    fn commit_exists(&self, _sha: &str) -> bool {
        true
    }

    fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
        Ok("parent".into())
    }

    fn has_changes(&self, _from: &str, _to: &str, _pathspec: &[&str]) -> Result<bool, AppError> {
        Ok(false)
    }

    fn run_command(&self, _args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
        Ok(String::new())
    }

    fn fetch(&self, _remote: &str) -> Result<(), AppError> {
        Ok(())
    }

    fn checkout_branch(&self, name: &str, create: bool) -> Result<(), AppError> {
        if create {
            self.branches_created.lock().unwrap().push(name.to_string());
        }
        *self.current_branch.lock().unwrap() = name.to_string();
        Ok(())
    }

    fn push_branch(&self, _name: &str, _force: bool) -> Result<(), AppError> {
        Ok(())
    }

    fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
        Ok(true)
    }

    fn commit_files(&self, _msg: &str, files: &[&Path]) -> Result<String, AppError> {
        let mut committed = self.committed_files.lock().unwrap();
        for f in files {
            committed.push(f.to_path_buf());
        }
        Ok("fake-sha".into())
    }
}
