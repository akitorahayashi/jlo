mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn init_creates_jules_directory() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .jules/"));

    ctx.assert_jules_exists();
    assert!(ctx.read_version().is_some());
    ctx.assert_layer_structure_exists();
    ctx.assert_all_builtin_roles_exist();
    ctx.assert_exchange_structure_exists();
    ctx.assert_events_structure_exists();
    ctx.assert_issues_directory_exists();
    ctx.assert_contracts_exist();
}

#[test]
#[serial]
fn init_fails_if_jules_exists() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli().arg("init").assert().failure().stderr(predicate::str::contains("already exists"));
}

#[test]
#[serial]
fn template_creates_new_role() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["template", "-l", "observers", "-n", "custom-role"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created new role"));

    ctx.assert_role_in_layer_exists("observers", "custom-role");
}

#[test]
#[serial]
fn template_fails_for_invalid_layer() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["template", "-l", "invalid", "-n", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid layer"));
}

#[test]
#[serial]
fn template_fails_for_existing_role() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["template", "-l", "observers", "-n", "taxonomy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
#[serial]
fn template_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["template", "-l", "observers", "-n", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No .jules/"));
}

#[test]
#[serial]
fn version_flag_works() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
#[serial]
fn help_lists_visible_aliases() {
    let ctx = TestContext::new();

    ctx.cli().arg("--help").assert().success().stdout(
        predicate::str::contains("[aliases: i]").and(predicate::str::contains("[aliases: tp]")),
    );
}

// =============================================================================
// Setup Command Tests
// =============================================================================

#[test]
#[serial]
fn init_creates_setup_structure() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .jules/"));

    assert!(ctx.work_dir().join(".jules/setup").exists());
    assert!(ctx.work_dir().join(".jules/setup/tools.yml").exists());
    assert!(ctx.work_dir().join(".jules/setup/.gitignore").exists());
}

#[test]
#[serial]
fn setup_gen_requires_init() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "gen"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Setup not initialized"));
}

#[test]
#[serial]
fn setup_gen_produces_script() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    // Write tools config
    let tools_yml = ctx.work_dir().join(".jules/setup/tools.yml");
    std::fs::write(&tools_yml, "tools:\n  - just\n").unwrap();

    ctx.cli()
        .args(["setup", "gen"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated install.sh"));

    assert!(ctx.work_dir().join(".jules/setup/install.sh").exists());
    assert!(ctx.work_dir().join(".jules/setup/env.toml").exists());
}

#[test]
#[serial]
fn setup_list_shows_components() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available components:"))
        .stdout(predicate::str::contains("just"))
        .stdout(predicate::str::contains("swift"))
        .stdout(predicate::str::contains("uv"));
}

#[test]
#[serial]
fn setup_list_detail_shows_info() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list", "--detail", "just"])
        .assert()
        .success()
        .stdout(predicate::str::contains("just:"))
        .stdout(predicate::str::contains("Install Script:"));
}

#[test]
#[serial]
fn setup_list_detail_not_found() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list", "--detail", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// Run Implementers Tests
// =============================================================================

#[test]
#[serial]
fn run_implementers_requires_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    ctx.cli()
        .args(["run", "implementers"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
#[serial]
fn run_planners_requires_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    ctx.cli()
        .args(["run", "planners"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
#[serial]
fn run_implementers_with_missing_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    ctx.cli()
        .args(["run", "implementers", ".jules/workstreams/generic/issues/nonexistent.yml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue file not found"));
}

#[test]
#[serial]
fn run_implementers_dry_run_with_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    // Create a test issue file in workstreams
    let issue_dir = ctx.work_dir().join(".jules/workstreams/generic/issues/medium");
    std::fs::create_dir_all(&issue_dir).unwrap();
    let issue_path = issue_dir.join("test_issue.yml");
    std::fs::write(
        &issue_path,
        "fingerprint: test_issue\nid: test_issue\ntitle: Test Issue\nstatus: open\n",
    )
    .unwrap();

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "run",
            "implementers",
            ".jules/workstreams/generic/issues/medium/test_issue.yml",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry Run: Local Dispatch"))
        .stdout(predicate::str::contains("Would dispatch workflow"));
}

#[test]
#[serial]
fn run_planners_dry_run_with_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    // Create a test issue file in workstreams
    let issue_dir = ctx.work_dir().join(".jules/workstreams/generic/issues/medium");
    std::fs::create_dir_all(&issue_dir).unwrap();
    let issue_path = issue_dir.join("test_issue.yml");
    std::fs::write(
        &issue_path,
        "fingerprint: test_issue\nid: test_issue\ntitle: Test Issue\nstatus: open\nrequires_deep_analysis: true\n",
    )
    .unwrap();

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "run",
            "planners",
            ".jules/workstreams/generic/issues/medium/test_issue.yml",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry Run: Local Dispatch"))
        .stdout(predicate::str::contains("Would dispatch workflow"));
}

// =============================================================================
// Update Command Tests
// =============================================================================

#[test]
#[serial]
fn update_requires_workspace() {
    let ctx = TestContext::new();

    ctx.cli().args(["update"]).assert().failure().stderr(predicate::str::contains("No .jules/"));
}

#[test]
#[serial]
fn update_dry_run_shows_plan() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    // Simulate an older version to trigger update logic
    let version_file = ctx.work_dir().join(".jules").join(".jlo-version");
    std::fs::write(&version_file, "0.0.0").expect("write version");

    ctx.cli()
        .args(["update", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry Run"));
}

#[test]
#[serial]
fn update_noop_when_current() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    ctx.cli().args(["update"]).assert().success().stdout(predicate::str::contains("already"));
}

#[test]
#[serial]
fn update_alias_works() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().success();

    ctx.cli().args(["u", "--dry-run"]).assert().success();
}
