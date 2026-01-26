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
    ctx.assert_all_builtin_roles_exist();
    ctx.assert_global_reports_exists();
    ctx.assert_issues_structure_exists();
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
        .stdout(predicate::str::contains("Workspace already up to date"));
}

#[test]
#[serial]
fn update_succeeds_if_modified() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();
    ctx.modify_jo_file("README.md", "MODIFIED");

    ctx.cli()
        .arg("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("Refreshed jo-managed files"));
}

#[test]
#[serial]
fn update_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli().arg("update").assert().failure().stderr(predicate::str::contains("No .jules/"));
}

#[test]
#[serial]
fn role_outputs_role_config() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .arg("role")
        .write_stdin("taxonomy\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("role: taxonomy"));

    ctx.assert_role_exists("taxonomy");
}

#[test]
#[serial]
fn role_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("role")
        .write_stdin("taxonomy\n")
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
        .arg("role")
        .write_stdin("invalid/id\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Role 'invalid/id' not found"));
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
            .and(predicate::str::contains("[aliases: r]")),
    );
}
