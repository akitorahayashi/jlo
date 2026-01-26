mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn user_can_init_and_select_role() {
    let ctx = TestContext::new();

    // Initialize workspace
    ctx.cli().arg("init").assert().success();

    // All 4 built-in roles should exist after init
    ctx.assert_all_builtin_roles_exist();

    // Select a role and get its config
    ctx.cli()
        .arg("role")
        .write_stdin("taxonomy\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("role: taxonomy"));
}

#[test]
#[serial]
fn user_can_update_after_modifications() {
    let ctx = TestContext::new();

    // Initialize workspace
    ctx.cli().arg("init").assert().success();

    // Modify a jo-managed file
    ctx.modify_jo_file("README.md", "MODIFIED CONTENT");

    // Update succeeds even with modifications
    ctx.cli()
        .arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("Refreshed jo-managed files"));
}

#[test]
#[serial]
fn user_can_use_command_aliases() {
    let ctx = TestContext::new();

    // Use 'i' alias for init
    ctx.cli().arg("i").assert().success();

    // Use 'r' alias for role
    ctx.cli().arg("r").write_stdin("taxonomy\n").assert().success();

    // Use 'u' alias for update
    ctx.cli().arg("u").assert().success();
}

#[test]
#[serial]
fn init_creates_complete_v1_structure() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    // Verify v1 structure
    ctx.assert_jules_exists();
    ctx.assert_global_reports_exists();
    ctx.assert_issues_structure_exists();
    ctx.assert_all_builtin_roles_exist();

    // Verify PM has policy.md
    let pm_policy = ctx.jules_path().join("roles/pm/policy.md");
    assert!(pm_policy.exists(), "PM should have policy.md");
}
