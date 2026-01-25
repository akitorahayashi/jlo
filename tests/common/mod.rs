//! Shared testing utilities for jo CLI tests.

use assert_cmd::Command;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Testing harness providing an isolated environment for CLI exercises.
#[allow(dead_code)]
pub struct TestContext {
    root: TempDir,
    work_dir: PathBuf,
    original_home: Option<OsString>,
    original_cwd: PathBuf,
}

#[allow(dead_code)]
impl TestContext {
    /// Create a new isolated environment.
    pub fn new() -> Self {
        let root = TempDir::new().expect("Failed to create temp directory for tests");
        let work_dir = root.path().join("work");
        fs::create_dir_all(&work_dir).expect("Failed to create test work directory");

        let original_home = env::var_os("HOME");
        let original_cwd = env::current_dir().expect("Failed to get current directory");

        unsafe {
            env::set_var("HOME", root.path());
        }

        Self { root, work_dir, original_home, original_cwd }
    }

    /// Absolute path to the emulated `$HOME` directory.
    pub fn home(&self) -> &Path {
        self.root.path()
    }

    /// Path to the workspace directory used for CLI invocations.
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Build a command for invoking the compiled `jo` binary within the default workspace.
    pub fn cli(&self) -> Command {
        self.cli_in(self.work_dir())
    }

    /// Build a command for invoking the compiled `jo` binary within a custom directory.
    pub fn cli_in<P: AsRef<Path>>(&self, dir: P) -> Command {
        let mut cmd = Command::cargo_bin("jo").expect("Failed to locate jo binary");
        cmd.current_dir(dir.as_ref()).env("HOME", self.home());
        cmd
    }

    /// Path to the .jules directory in the work directory.
    pub fn jules_path(&self) -> PathBuf {
        self.work_dir.join(".jules")
    }

    /// Assert that .jules directory exists.
    pub fn assert_jules_exists(&self) {
        assert!(self.jules_path().exists(), ".jules directory should exist");
    }

    /// Assert that .jules directory does not exist.
    pub fn assert_jules_not_exists(&self) {
        assert!(!self.jules_path().exists(), ".jules directory should not exist");
    }

    /// Assert that a role directory exists.
    pub fn assert_role_exists(&self, role_id: &str) {
        let role_path = self.jules_path().join("roles").join(role_id);
        assert!(role_path.exists(), "Role directory should exist at {}", role_path.display());
        assert!(role_path.join("charter.md").exists(), "Role charter should exist");
        assert!(role_path.join("direction.md").exists(), "Role direction should exist");
    }

    /// Read the .jo-version file.
    pub fn read_version(&self) -> Option<String> {
        let version_path = self.jules_path().join(".jo-version");
        if version_path.exists() {
            Some(
                fs::read_to_string(version_path)
                    .expect("Failed to read version")
                    .trim()
                    .to_string(),
            )
        } else {
            None
        }
    }

    /// Modify a jo-managed file for testing.
    pub fn modify_jo_file(&self, relative_path: &str, content: &str) {
        let path = self.jules_path().join(relative_path);
        fs::write(&path, content).expect("Failed to modify file");
    }

    /// Execute a closure after temporarily switching into the work directory.
    pub fn with_work_dir<F, R>(&self, action: F) -> R
    where
        F: FnOnce() -> R,
    {
        let original = env::current_dir().expect("Failed to capture current dir");
        env::set_current_dir(&self.work_dir).expect("Failed to switch current dir");
        let result = action();
        env::set_current_dir(original).expect("Failed to restore current dir");
        result
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Restore original CWD first (in case we're still in the temp dir)
        let _ = env::set_current_dir(&self.original_cwd);

        match &self.original_home {
            Some(value) => unsafe {
                env::set_var("HOME", value);
            },
            None => unsafe {
                env::remove_var("HOME");
            },
        }
    }
}
