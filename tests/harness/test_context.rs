//! Shared testing harness for `jlo` integration tests.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use toml::Value;

/// Testing harness providing an isolated environment for CLI exercises.
pub(crate) struct TestContext {
    root: TempDir,
    work_dir: PathBuf,
}

impl TestContext {
    /// Create a new isolated environment.
    pub(crate) fn new() -> Self {
        let root = TempDir::new().expect("Failed to create temp directory for tests");
        let work_dir = root.path().join("work");
        fs::create_dir_all(&work_dir).expect("Failed to create test work directory");

        // Initialize git repo on a control branch (not 'jules').
        // Explicitly set initial branch to 'main' to avoid default configuration dependency.
        let output = std::process::Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(&work_dir)
            .output()
            .expect("Failed to git init");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Configure git user/email for commits
        let output = std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&work_dir)
            .output()
            .expect("Failed to configure git user.name");
        assert!(output.status.success());

        let output = std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&work_dir)
            .output()
            .expect("Failed to configure git user.email");
        assert!(output.status.success());

        Self { root, work_dir }
    }

    /// Checkout a git branch in the test repo.
    pub(crate) fn git_checkout_branch(&self, branch: &str, create: bool) {
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
    pub(crate) fn home(&self) -> &Path {
        self.root.path()
    }

    /// Path to the workspace directory used for CLI invocations.
    pub(crate) fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Build a command for invoking the compiled `jlo` binary within the default workspace.
    pub(crate) fn cli(&self) -> Command {
        self.cli_in(self.work_dir())
    }

    /// Build a command for invoking the compiled `jlo` binary within a custom directory.
    pub(crate) fn cli_in<P: AsRef<Path>>(&self, dir: P) -> Command {
        let mut cmd = Command::cargo_bin("jlo").expect("Failed to locate jlo binary");
        cmd.current_dir(dir.as_ref()).env("HOME", self.home());
        cmd
    }

    /// Run `jlo init --remote` and assert success.
    pub(crate) fn init_remote(&self) {
        self.cli().args(["init", "--remote"]).assert().success();
    }

    /// Run `jlo init --self-hosted` and assert success.
    pub(crate) fn init_self_hosted(&self) {
        self.cli().args(["init", "--self-hosted"]).assert().success();
    }

    /// Run bootstrap subcommands and assert success.
    pub(crate) fn workflow_bootstrap(&self) {
        self.cli().args(["workflow", "bootstrap", "managed-files"]).assert().success();
        self.cli().args(["workflow", "bootstrap", "workstations"]).assert().success();
    }

    /// Initialize the workspace and run bootstrap subcommands for `.jules/`.
    pub(crate) fn init_remote_and_bootstrap(&self) {
        self.init_remote();
        self.workflow_bootstrap();
    }

    /// Path to the `.jlo/` directory in the work directory.
    pub(crate) fn jlo_path(&self) -> PathBuf {
        self.work_dir.join(".jlo")
    }

    /// Assert that `.jlo/` directory exists.
    pub(crate) fn assert_jlo_exists(&self) {
        assert!(self.jlo_path().exists(), ".jlo directory should exist");
    }

    /// Assert that `.jlo/` directory does not exist.
    pub(crate) fn assert_jlo_not_exists(&self) {
        assert!(!self.jlo_path().exists(), ".jlo directory should not exist");
    }

    /// Path to the `.jules/` directory in the work directory.
    pub(crate) fn jules_path(&self) -> PathBuf {
        self.work_dir.join(".jules")
    }

    /// Assert that `.jules/` directory exists.
    pub(crate) fn assert_jules_exists(&self) {
        assert!(self.jules_path().exists(), ".jules directory should exist");
    }

    /// Assert that `.jules/` directory does not exist.
    pub(crate) fn assert_jules_not_exists(&self) {
        assert!(!self.jules_path().exists(), ".jules directory should not exist");
    }

    /// Path to the `.jules/layers` directory.
    pub(crate) fn layers_path(&self) -> PathBuf {
        self.jules_path().join("layers")
    }

    /// Path to the `.jules/exchange` directory.
    pub(crate) fn exchange_path(&self) -> PathBuf {
        self.jules_path().join("exchange")
    }

    /// Path to the `.jules/exchange/events` directory.
    pub(crate) fn events_path(&self) -> PathBuf {
        self.exchange_path().join("events")
    }

    /// Path to the `.jules/exchange/requirements` directory.
    pub(crate) fn requirements_path(&self) -> PathBuf {
        self.exchange_path().join("requirements")
    }

    /// Assert that layer directories exist.
    pub(crate) fn assert_layer_structure_exists(&self) {
        let roles_path = self.layers_path();
        assert!(roles_path.join("narrator").exists(), "narrator layer should exist");
        assert!(roles_path.join("observers").exists(), "observers layer should exist");
        assert!(roles_path.join("decider").exists(), "decider layer should exist");
        assert!(roles_path.join("planner").exists(), "planner layer should exist");
        assert!(roles_path.join("implementer").exists(), "implementer layer should exist");
        assert!(roles_path.join("integrator").exists(), "integrator layer should exist");
    }

    /// Assert that the narrator layer exists with correct structure.
    pub(crate) fn assert_narrator_exists(&self) {
        self.assert_single_role_layer_exists("narrator");
        let narrator_path = self.layers_path().join("narrator");
        assert!(
            narrator_path.join("schemas").join("changes.yml").exists(),
            "narrator schemas/changes.yml should exist"
        );
    }

    /// Assert that a role exists within a specific layer in `.jlo/` (control plane).
    pub(crate) fn assert_role_in_layer_exists(&self, layer: &str, role_id: &str) {
        let role_path = self.jlo_path().join("roles").join(layer).join(role_id);
        assert!(role_path.exists(), "Role directory should exist at {}", role_path.display());
        assert!(role_path.join("role.yml").exists(), "Role role.yml should exist");
    }

    /// Assert that a role directory exists (searches multi-role layers) in `.jlo/`.
    pub(crate) fn assert_role_exists(&self, role_id: &str) {
        let layers = ["observers", "innovators"];
        let found = layers.iter().any(|layer| {
            self.jlo_path().join("roles").join(layer).join(role_id).join("role.yml").exists()
        });
        assert!(found, "Role {} should exist in some layer in .jlo/", role_id);
    }

    /// Assert that the events directory structure exists (flat exchange).
    pub(crate) fn assert_events_structure_exists(&self) {
        let events_path = self.events_path();
        assert!(events_path.exists(), "exchange/events should exist");
        assert!(events_path.join("pending").exists(), "exchange/events/pending should exist");
        assert!(events_path.join("decided").exists(), "exchange/events/decided should exist");
    }

    /// Assert that the requirements directory exists (flat exchange).
    pub(crate) fn assert_requirements_directory_exists(&self) {
        assert!(self.requirements_path().exists(), "exchange/requirements directory should exist");
    }

    /// Assert that flat exchange directory structure exists.
    pub(crate) fn assert_exchange_structure_exists(&self) {
        let exchange = self.exchange_path();
        assert!(exchange.exists(), "exchange directory should exist");
        assert!(exchange.join("events").exists(), "exchange/events should exist");
        assert!(exchange.join("events/pending").exists(), "exchange/events/pending should exist");
        assert!(exchange.join("events/decided").exists(), "exchange/events/decided should exist");
        assert!(exchange.join("requirements").exists(), "exchange/requirements should exist");
        assert!(exchange.join("proposals").exists(), "exchange/proposals should exist");
    }

    /// Assert that `contracts.yml` exists in each layer directory.
    pub(crate) fn assert_contracts_exist(&self) {
        let roles_path = self.layers_path();
        assert!(
            roles_path.join("narrator/contracts.yml").exists(),
            "narrator/contracts.yml should exist"
        );
        assert!(
            roles_path.join("observers/contracts.yml").exists(),
            "observers/contracts.yml should exist"
        );
        assert!(
            roles_path.join("decider/contracts.yml").exists(),
            "decider/contracts.yml should exist"
        );
        assert!(
            roles_path.join("planner/contracts.yml").exists(),
            "planner/contracts.yml should exist"
        );
        assert!(
            roles_path.join("implementer/contracts.yml").exists(),
            "implementer/contracts.yml should exist"
        );
        assert!(
            roles_path.join("integrator/contracts.yml").exists(),
            "integrator/contracts.yml should exist"
        );
    }

    /// Assert that default scheduled roles exist in `.jlo/config.toml`.
    pub(crate) fn assert_default_scheduled_roles_exist(&self) {
        let content =
            fs::read_to_string(self.jlo_path().join("config.toml")).expect("read .jlo/config.toml");
        let value: Value = toml::from_str(&content).expect("parse .jlo/config.toml");

        let observers = value
            .get("observers")
            .and_then(|section| section.get("roles"))
            .and_then(|roles| roles.as_array())
            .cloned()
            .unwrap_or_default();
        let observer_names: Vec<String> = observers
            .into_iter()
            .filter_map(|role| {
                role.get("name").and_then(|name| name.as_str()).map(|name| name.to_string())
            })
            .collect();
        for expected in ["taxonomy", "data_arch", "structural_arch", "qa", "cov", "consistency"] {
            assert!(
                observer_names.iter().any(|name| name == expected),
                "missing default observer role '{}' in .jlo/config.toml",
                expected
            );
        }

        let innovators = value
            .get("innovators")
            .and_then(|section| section.get("roles"))
            .and_then(|roles| roles.as_array())
            .cloned()
            .unwrap_or_default();
        let innovator_names: Vec<String> = innovators
            .into_iter()
            .filter_map(|role| {
                role.get("name").and_then(|name| name.as_str()).map(|name| name.to_string())
            })
            .collect();
        for expected in ["recruiter", "leverage_architect"] {
            assert!(
                innovator_names.iter().any(|name| name == expected),
                "missing default innovator role '{}' in .jlo/config.toml",
                expected
            );
        }

        // Single-role layers have contracts.yml directly in layer directory.
        self.assert_single_role_layer_exists("narrator");
        self.assert_single_role_layer_exists("decider");
        self.assert_single_role_layer_exists("planner");
        self.assert_single_role_layer_exists("implementer");
    }

    /// Assert that a single-role layer exists with the correct structure.
    pub(crate) fn assert_single_role_layer_exists(&self, layer: &str) {
        let layer_path = self.layers_path().join(layer);
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

    /// Read the `.jlo-version` file from the `.jlo/` control plane.
    pub(crate) fn read_jlo_version(&self) -> Option<String> {
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

    /// Read the `.jlo-version` file from the `.jules/` runtime workspace.
    pub(crate) fn read_version(&self) -> Option<String> {
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
