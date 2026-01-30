//! Shared testing utilities for jlo CLI tests.

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

        // Initialize git repo and switch to jules branch to satisfy init requirements
        let output = std::process::Command::new("git")
            .arg("init")
            .current_dir(&work_dir)
            .output()
            .expect("Failed to git init");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = std::process::Command::new("git")
            .args(["checkout", "-b", "jules"])
            .current_dir(&work_dir)
            .output()
            .expect("Failed to checkout jules branch");
        assert!(
            output.status.success(),
            "git checkout failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

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

    /// Build a command for invoking the compiled `jlo` binary within the default workspace.
    pub fn cli(&self) -> Command {
        self.cli_in(self.work_dir())
    }

    /// Build a command for invoking the compiled `jlo` binary within a custom directory.
    pub fn cli_in<P: AsRef<Path>>(&self, dir: P) -> Command {
        let mut cmd = Command::cargo_bin("jlo").expect("Failed to locate jlo binary");
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

        // All roles have prompt.yml
        assert!(role_path.join("prompt.yml").exists(), "Role prompt.yml should exist");

        // Only observers have role.yml
        if layer == "observers" {
            assert!(role_path.join("role.yml").exists(), "Observer role.yml should exist");
        } else {
            assert!(
                !role_path.join("role.yml").exists(),
                "Non-observer should not have role.yml (behavior defined in archetype)"
            );
        }
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
        assert!(role_path.join("prompt.yml").exists(), "Role prompt.yml should exist");
        assert!(role_path.join("notes").exists(), "Role notes directory should exist");
    }

    /// Assert that the events directory structure exists (workstream-based).
    pub fn assert_events_structure_exists(&self) {
        let events_path = self.jules_path().join("workstreams/generic/events");
        assert!(events_path.exists(), "workstreams/generic/events should exist");
    }

    /// Assert that the issues directory exists (workstream-based).
    pub fn assert_issues_directory_exists(&self) {
        let issues_path = self.jules_path().join("workstreams/generic/issues");
        assert!(issues_path.exists(), "workstreams/generic/issues directory should exist");
    }

    /// Assert that workstreams directory structure exists.
    pub fn assert_workstreams_structure_exists(&self) {
        let ws_path = self.jules_path().join("workstreams");
        assert!(ws_path.exists(), "workstreams directory should exist");
        assert!(ws_path.join("generic").exists(), "generic workstream should exist");
        assert!(ws_path.join("generic/events").exists(), "generic/events should exist");
        assert!(ws_path.join("generic/issues").exists(), "generic/issues should exist");
    }

    /// Assert that exchange directory structure exists (for backward compatibility, now checks workstreams).
    pub fn assert_exchange_structure_exists(&self) {
        self.assert_workstreams_structure_exists();
    }

    /// Assert that contracts.yml exists in each layer directory.
    pub fn assert_contracts_exist(&self) {
        let roles_path = self.jules_path().join("roles");
        assert!(
            roles_path.join("observers/contracts.yml").exists(),
            "observers/contracts.yml should exist"
        );
        assert!(
            roles_path.join("deciders/contracts.yml").exists(),
            "deciders/contracts.yml should exist"
        );
        assert!(
            roles_path.join("planners/contracts.yml").exists(),
            "planners/contracts.yml should exist"
        );
        assert!(
            roles_path.join("implementers/contracts.yml").exists(),
            "implementers/contracts.yml should exist"
        );
    }

    /// Assert that feedbacks directories exist for all observer roles.
    pub fn assert_feedbacks_directories_exist(&self) {
        let observers = ["taxonomy", "data_arch", "qa"];
        for role in &observers {
            let feedbacks_path =
                self.jules_path().join("roles").join("observers").join(role).join("feedbacks");
            assert!(feedbacks_path.exists(), "feedbacks directory should exist for {}", role);
        }
    }

    /// Assert that all built-in roles exist in their correct layers.
    pub fn assert_all_builtin_roles_exist(&self) {
        self.assert_role_in_layer_exists("observers", "taxonomy");
        self.assert_role_in_layer_exists("observers", "data_arch");
        self.assert_role_in_layer_exists("observers", "qa");
        self.assert_role_in_layer_exists("deciders", "triage_generic");
        self.assert_role_in_layer_exists("planners", "specifier_global");
        self.assert_role_in_layer_exists("implementers", "executor_global");
    }

    /// Read the .jlo-version file.
    pub fn read_version(&self) -> Option<String> {
        let version_path = self.jules_path().join(".jlo-version");
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
