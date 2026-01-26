mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn user_can_init_and_create_role() {
    let ctx = TestContext::new();

    // Initialize workspace
    ctx.cli().arg("init").assert().success();

    // Create a role
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
