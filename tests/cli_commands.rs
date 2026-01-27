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
    ctx.assert_events_structure_exists();
    ctx.assert_issues_directory_exists();
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
fn assign_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["assign", "taxonomy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No .jules/"));
}

#[test]
#[serial]
fn assign_fails_for_unknown_role() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["assign", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
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
        predicate::str::contains("[aliases: i]")
            .and(predicate::str::contains("[aliases: a]"))
            .and(predicate::str::contains("[aliases: tp]")),
    );
}
