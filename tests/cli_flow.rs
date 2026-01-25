mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn user_can_init_create_role_and_session() {
    let ctx = TestContext::new();

    // Initialize workspace
    ctx.cli().arg("init").assert().success();

    // Check status shows up to date
    ctx.cli().arg("status").assert().success().stdout(predicate::str::contains("up to date"));

    // Create a role
    ctx.cli()
        .arg("role")
        .write_stdin("taxonomy\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created role 'taxonomy'"));

    // Create a session for that role
    ctx.cli()
        .args(["session", "taxonomy", "--slug", "initial-analysis"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created session"));

    // Status still shows up to date
    ctx.cli().arg("status").assert().success().stdout(predicate::str::contains("up to date"));
}

#[test]
#[serial]
fn user_can_update_after_modifications() {
    let ctx = TestContext::new();

    // Initialize workspace
    ctx.cli().arg("init").assert().success();

    // Modify a jo-managed file
    ctx.modify_jo_file(".jo/policy/contract.md", "MODIFIED CONTENT");

    // Status shows modifications
    ctx.cli()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified jo-managed files"));

    // Update without force fails
    ctx.cli().arg("update").assert().failure();

    // Update with force succeeds
    ctx.cli()
        .args(["update", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Refreshed jo-managed files"));

    // Status no longer shows modifications
    ctx.cli().arg("status").assert().success().stdout(
        predicate::str::contains("up to date").and(predicate::str::contains("Modified").not()),
    );
}

#[test]
#[serial]
fn user_can_use_command_aliases() {
    let ctx = TestContext::new();

    // Use 'i' alias for init
    ctx.cli().arg("i").assert().success();

    // Use 'st' alias for status
    ctx.cli().arg("st").assert().success();

    // Use 'r' alias for role
    ctx.cli().arg("r").write_stdin("taxonomy\n").assert().success();

    // Use 's' alias for session
    ctx.cli().args(["s", "taxonomy"]).assert().success();

    // Use 'u' alias for update
    ctx.cli().arg("u").assert().success();
}
