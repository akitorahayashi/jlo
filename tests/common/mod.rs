//! Shared testing utilities for jlo CLI tests.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Testing harness providing an isolated environment for CLI exercises.
#[allow(dead_code)]
pub struct TestContext {
    root: TempDir,
    work_dir: PathBuf,
}

#[allow(dead_code)]
impl TestContext {
    /// Create a new isolated environment.
    pub fn new() -> Self {
        let root = TempDir::new().expect("Failed to create temp directory for tests");
        let work_dir = root.path().join("work");
        fs::create_dir_all(&work_dir).expect("Failed to create test work directory");

        // Initialize git repo on a control branch (not 'jules')
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

        Self { root, work_dir }
    }

    /// Checkout a git branch in the test repo.
    pub fn git_checkout_branch(&self, branch: &str, create: bool) {
        let mut args = vec!["checkout"];
        if create {
            args.push("-b");
        }
        args.push(branch);

        let output = std::process::Command::new("git")
            .args(&args)
            .current_dir(&self.work_dir)
            .output()
            .expect("Failed to run git checkout");
        assert!(
            output.status.success(),
            "git checkout failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
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

    /// Path to the .jlo directory in the work directory.
    pub fn jlo_path(&self) -> PathBuf {
        self.work_dir.join(".jlo")
    }

    /// Assert that .jlo directory exists.
    pub fn assert_jlo_exists(&self) {
        assert!(self.jlo_path().exists(), ".jlo directory should exist");
    }

    /// Assert that .jlo directory does not exist.
    pub fn assert_jlo_not_exists(&self) {
        assert!(!self.jlo_path().exists(), ".jlo directory should not exist");
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
        assert!(roles_path.join("narrator").exists(), "narrator layer should exist");
        assert!(roles_path.join("observers").exists(), "observers layer should exist");
        assert!(roles_path.join("deciders").exists(), "deciders layer should exist");
        assert!(roles_path.join("planners").exists(), "planners layer should exist");
        assert!(roles_path.join("implementers").exists(), "implementers layer should exist");
    }

    /// Assert that the narrator layer exists with correct structure.
    pub fn assert_narrator_exists(&self) {
        self.assert_single_role_layer_exists("narrator");
        let narrator_path = self.jules_path().join("roles").join("narrator");
        assert!(
            narrator_path.join("schemas").join("changes.yml").exists(),
            "narrator schemas/changes.yml should exist"
        );
    }

    /// Assert that a role exists within a specific layer in .jlo/ (Control Plane).
    pub fn assert_role_in_layer_exists(&self, layer: &str, role_id: &str) {
        let role_path = self.jlo_path().join("roles").join(layer).join(role_id);
        assert!(role_path.exists(), "Role directory should exist at {}", role_path.display());
        assert!(role_path.join("role.yml").exists(), "Role role.yml should exist");
    }

    /// Assert that a role directory exists (searches multi-role layers) in .jlo/.
    pub fn assert_role_exists(&self, role_id: &str) {
        let layers = ["observers", "innovators"];
        let found = layers.iter().any(|layer| {
            self.jlo_path().join("roles").join(layer).join(role_id).join("role.yml").exists()
        });
        assert!(found, "Role {} should exist in some layer in .jlo/", role_id);
    }

    /// Assert that the events directory structure exists (flat exchange).
    pub fn assert_events_structure_exists(&self) {
        let events_path = self.jules_path().join("exchange/events");
        assert!(events_path.exists(), "exchange/events should exist");
        assert!(events_path.join("pending").exists(), "exchange/events/pending should exist");
        assert!(events_path.join("decided").exists(), "exchange/events/decided should exist");
    }

    /// Assert that the issues directory exists (flat exchange).
    pub fn assert_issues_directory_exists(&self) {
        let issues_path = self.jules_path().join("exchange/issues");
        assert!(issues_path.exists(), "exchange/issues directory should exist");
    }

    /// Assert that flat exchange directory structure exists.
    pub fn assert_exchange_structure_exists(&self) {
        let exchange = self.jules_path().join("exchange");
        assert!(exchange.exists(), "exchange directory should exist");
        assert!(exchange.join("events").exists(), "exchange/events should exist");
        assert!(exchange.join("events/pending").exists(), "exchange/events/pending should exist");
        assert!(exchange.join("events/decided").exists(), "exchange/events/decided should exist");
        assert!(exchange.join("issues").exists(), "exchange/issues should exist");
        assert!(exchange.join("innovators").exists(), "exchange/innovators should exist");
    }

    /// Assert that contracts.yml exists in each layer directory.
    pub fn assert_contracts_exist(&self) {
        let roles_path = self.jules_path().join("roles");
        assert!(
            roles_path.join("narrator/contracts.yml").exists(),
            "narrator/contracts.yml should exist"
        );
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

    /// Assert that default scheduled roles exist in their correct layers.
    pub fn assert_default_scheduled_roles_exist(&self) {
        self.assert_role_in_layer_exists("observers", "taxonomy");
        self.assert_role_in_layer_exists("observers", "data_arch");
        self.assert_role_in_layer_exists("observers", "structural_arch");
        self.assert_role_in_layer_exists("observers", "qa");
        self.assert_role_in_layer_exists("observers", "cov");
        self.assert_role_in_layer_exists("observers", "consistency");
        self.assert_role_in_layer_exists("innovators", "recruiter");

        // Single-role layers have contracts.yml directly in layer directory
        self.assert_single_role_layer_exists("narrator");
        self.assert_single_role_layer_exists("deciders");
        self.assert_single_role_layer_exists("planners");
        self.assert_single_role_layer_exists("implementers");
    }

    /// Assert that a single-role layer exists with the correct structure.
    pub fn assert_single_role_layer_exists(&self, layer: &str) {
        let layer_path = self.jules_path().join("roles").join(layer);
        assert!(layer_path.exists(), "Layer directory should exist at {}", layer_path.display());

        assert!(
            layer_path.join("contracts.yml").exists(),
            "Layer contracts.yml should exist at {}",
            layer_path.join("contracts.yml").display()
        );

        assert!(
            layer_path.join("tasks").exists(),
            "tasks/ directory should exist in layer {}",
            layer
        );
    }

    /// Read the .jlo-version file from the .jlo/ control plane.
    pub fn read_jlo_version(&self) -> Option<String> {
        let version_path = self.jlo_path().join(".jlo-version");
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

    /// Read the .jlo-version file from the .jules/ runtime workspace.
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
}
