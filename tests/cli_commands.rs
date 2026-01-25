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
fn init_force_overwrites_existing() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli().args(["init", "--force"]).assert().success();
}

#[test]
#[serial]
fn update_updates_jo_managed_files() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("Refreshed jo-managed files"));
}

#[test]
#[serial]
fn update_fails_if_modified_without_force() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();
    ctx.modify_jo_file(".jo/policy/contract.md", "MODIFIED");

    ctx.cli()
        .arg("update")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Modified jo-managed files"));
}

#[test]
#[serial]
fn update_force_overwrites_modified() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();
    ctx.modify_jo_file(".jo/policy/contract.md", "MODIFIED");

    ctx.cli().args(["update", "--force"]).assert().success();
}

#[test]
#[serial]
fn update_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli().arg("update").assert().failure().stderr(predicate::str::contains("No .jules/"));
}

#[test]
#[serial]
fn status_shows_no_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("No .jules/ workspace"));
}

#[test]
#[serial]
fn status_shows_versions() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli().arg("status").assert().success().stdout(
        predicate::str::contains("jo version:")
            .and(predicate::str::contains("Workspace version:"))
            .and(predicate::str::contains("up to date")),
    );
}

#[test]
#[serial]
fn status_detects_modifications() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();
    ctx.modify_jo_file(".jo/policy/contract.md", "MODIFIED");

    ctx.cli()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified jo-managed files"));
}

#[test]
#[serial]
fn role_creates_role_directory() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["role", "value"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created role 'value'"));

    ctx.assert_role_exists("value");
}

#[test]
#[serial]
fn role_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["role", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No .jules/"));
}

#[test]
#[serial]
fn role_fails_for_invalid_id() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["role", "invalid/id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid role identifier"));
}

#[test]
#[serial]
fn session_creates_session_file() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();
    ctx.cli().args(["role", "value"]).assert().success();

    ctx.cli()
        .args(["session", "value", "--slug", "test-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created session"));
}

#[test]
#[serial]
fn session_fails_for_nonexistent_role() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["session", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Role 'nonexistent' not found"));
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
        predicate::str::contains("[aliases: i]")
            .and(predicate::str::contains("[aliases: u]"))
            .and(predicate::str::contains("[aliases: st]"))
            .and(predicate::str::contains("[aliases: r]"))
            .and(predicate::str::contains("[aliases: s]")),
    );
}
