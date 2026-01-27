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

    /// Assert that layer directories exist.
    pub fn assert_layer_structure_exists(&self) {
        let roles_path = self.jules_path().join("roles");
        assert!(roles_path.join("observers").exists(), "observers layer should exist");
        assert!(roles_path.join("deciders").exists(), "deciders layer should exist");
        assert!(roles_path.join("planners").exists(), "planners layer should exist");
        assert!(roles_path.join("implementers").exists(), "implementers layer should exist");
    }

    /// Assert that a role exists within a specific layer.
    pub fn assert_role_in_layer_exists(&self, layer: &str, role_id: &str) {
        let role_path = self.jules_path().join("roles").join(layer).join(role_id);
        assert!(role_path.exists(), "Role directory should exist at {}", role_path.display());
        assert!(role_path.join("role.yml").exists(), "Role role.yml should exist");
    }

    /// Assert that a role directory exists (legacy compatibility - searches all layers).
    pub fn assert_role_exists(&self, role_id: &str) {
        let layers = ["observers", "deciders", "planners", "implementers"];
        let found = layers.iter().any(|layer| {
            self.jules_path().join("roles").join(layer).join(role_id).join("role.yml").exists()
        });
        assert!(found, "Role {} should exist in some layer", role_id);
    }

    /// Assert that a worker role directory exists (role.yml + notes).
    pub fn assert_worker_role_exists(&self, role_id: &str) {
        // Worker roles are in observers layer
        let role_path = self.jules_path().join("roles").join("observers").join(role_id);
        assert!(role_path.exists(), "Role directory should exist at {}", role_path.display());
        assert!(role_path.join("role.yml").exists(), "Role role.yml should exist");
        assert!(role_path.join("notes").exists(), "Role notes directory should exist");
    }

    /// Assert that the events directory structure exists.
    pub fn assert_events_structure_exists(&self) {
        let events_path = self.jules_path().join("events");
        assert!(events_path.join("bugs").exists(), "events/bugs should exist");
        assert!(events_path.join("refacts").exists(), "events/refacts should exist");
        assert!(events_path.join("updates").exists(), "events/updates should exist");
        assert!(events_path.join("tests").exists(), "events/tests should exist");
        assert!(events_path.join("docs").exists(), "events/docs should exist");
    }

    /// Assert that the issues directory exists (flat layout).
    pub fn assert_issues_directory_exists(&self) {
        let issues_path = self.jules_path().join("issues");
        assert!(issues_path.exists(), "issues directory should exist");
    }

    /// Assert that all built-in roles exist in their correct layers.
    pub fn assert_all_builtin_roles_exist(&self) {
        self.assert_role_in_layer_exists("observers", "taxonomy");
        self.assert_role_in_layer_exists("observers", "data_arch");
        self.assert_role_in_layer_exists("observers", "qa");
        self.assert_role_in_layer_exists("deciders", "triage");
        self.assert_role_in_layer_exists("planners", "specifier");
        self.assert_role_in_layer_exists("implementers", "executor");
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
