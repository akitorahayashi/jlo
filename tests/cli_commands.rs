mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn add_command_persists_item() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("add")
        .arg("demo")
        .arg("--content")
        .arg("example value")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added item 'demo'"));

    ctx.assert_saved_item_contains("demo", "example value");
}

#[test]
#[serial]
fn list_command_outputs_items() {
    let ctx = TestContext::new();

    ctx.cli().args(["add", "first", "--content", "one"]).assert().success();
    ctx.cli().args(["add", "second", "--content", "two"]).assert().success();

    ctx.cli()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("first").and(predicate::str::contains("second")));
}

#[test]
#[serial]
fn delete_command_removes_item() {
    let ctx = TestContext::new();

    ctx.cli().args(["add", "temp", "--content", "value"]).assert().success();

    assert!(ctx.saved_item_path("temp").exists(), "Item should exist before delete");

    ctx.cli()
        .arg("delete")
        .arg("temp")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted item 'temp'"));

    assert!(!ctx.saved_item_path("temp").exists(), "Item should not exist after delete");
}

#[test]
#[serial]
fn delete_nonexistent_item_fails() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("delete")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Item 'nonexistent' was not found"));
}

#[test]
#[serial]
fn add_with_invalid_id_fails() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["add", "invalid/id", "--content", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid item identifier"));
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
        predicate::str::contains("[aliases: a]")
            .and(predicate::str::contains("[aliases: ls]"))
            .and(predicate::str::contains("[aliases: rm]")),
    );
}
