//! Repository source detection from git remote.

use crate::domain::AppError;
use crate::domain::configuration::run_config_parser;
use crate::ports::Git;

/// Detect the repository source from git remote or `GITHUB_REPOSITORY` env var.
pub fn detect_repository_source(git: &(impl Git + ?Sized)) -> Result<String, AppError> {
    let output = git.run_command(&["remote", "get-url", "origin"], None);

    if let Ok(url) = output
        && let Some(repo) = run_config_parser::parse_github_url(url.trim())
    {
        return Ok(format!("sources/github/{}", repo));
    }

    if let Ok(repo) = std::env::var("GITHUB_REPOSITORY") {
        return Ok(format!("sources/github/{}", repo));
    }

    Err(AppError::RepositoryDetectionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::path::Path;

    struct MockGit {
        remote_url: Option<String>,
        fail: bool,
    }

    impl Git for MockGit {
        fn get_head_sha(&self) -> Result<String, AppError> {
            Ok(String::new())
        }
        fn get_current_branch(&self) -> Result<String, AppError> {
            Ok(String::new())
        }
        fn commit_exists(&self, _sha: &str) -> bool {
            true
        }
        fn get_nth_ancestor(&self, _commit: &str, _n: usize) -> Result<String, AppError> {
            Ok(String::new())
        }
        fn has_changes(
            &self,
            _from: &str,
            _to: &str,
            _pathspec: &[&str],
        ) -> Result<bool, AppError> {
            Ok(false)
        }
        fn run_command(&self, args: &[&str], _cwd: Option<&Path>) -> Result<String, AppError> {
            if self.fail {
                return Err(AppError::InternalError("Mock git failure".into()));
            }
            if args.len() >= 3
                && args[0] == "remote"
                && args[1] == "get-url"
                && args[2] == "origin"
                && let Some(ref url) = self.remote_url
            {
                return Ok(url.clone());
            }
            Ok(String::new())
        }
        fn checkout_branch(&self, _branch: &str, _create: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn push_branch(&self, _branch: &str, _force: bool) -> Result<(), AppError> {
            Ok(())
        }
        fn commit_files(&self, _message: &str, _files: &[&Path]) -> Result<String, AppError> {
            Ok(String::new())
        }
        fn fetch(&self, _remote: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn delete_branch(&self, _branch: &str, _force: bool) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    struct EnvVarGuard {
        key: String,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set<K: Into<String>, V: AsRef<std::ffi::OsStr>>(key: K, value: V) -> Self {
            let key = key.into();
            let original = std::env::var_os(&key);
            unsafe { std::env::set_var(&key, value) };
            Self { key, original }
        }

        fn remove<K: Into<String>>(key: K) -> Self {
            let key = key.into();
            let original = std::env::var_os(&key);
            unsafe { std::env::remove_var(&key) };
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(original) = self.original.as_ref() {
                unsafe { std::env::set_var(&self.key, original) };
            } else {
                unsafe { std::env::remove_var(&self.key) };
            }
        }
    }

    #[test]
    #[serial]
    fn detects_github_ssh_url() {
        let _guard = EnvVarGuard::remove("GITHUB_REPOSITORY");
        let git =
            MockGit { remote_url: Some("git@github.com:owner/repo.git".to_string()), fail: false };
        let result = detect_repository_source(&git).expect("should succeed");
        assert_eq!(result, "sources/github/owner/repo");
    }

    #[test]
    #[serial]
    fn detects_github_https_url() {
        let _guard = EnvVarGuard::remove("GITHUB_REPOSITORY");
        let git = MockGit {
            remote_url: Some("https://github.com/owner/repo.git".to_string()),
            fail: false,
        };
        let result = detect_repository_source(&git).expect("should succeed");
        assert_eq!(result, "sources/github/owner/repo");
    }

    #[test]
    #[serial]
    fn detects_from_env_var_when_git_fails() {
        let _guard = EnvVarGuard::set("GITHUB_REPOSITORY", "env-owner/env-repo");
        let git = MockGit { remote_url: None, fail: true };
        let result = detect_repository_source(&git).expect("should succeed from env");
        assert_eq!(result, "sources/github/env-owner/env-repo");
    }

    #[test]
    #[serial]
    fn fails_when_both_fail() {
        let _guard = EnvVarGuard::remove("GITHUB_REPOSITORY");
        let git = MockGit { remote_url: None, fail: true };
        let result = detect_repository_source(&git);
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::RepositoryDetectionFailed)));
    }
}
