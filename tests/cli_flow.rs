mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn user_can_add_list_and_delete_items() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["add", "workflow", "--content", "example"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added item 'workflow'"));

    ctx.cli().arg("list").assert().success().stdout(predicate::str::contains("- workflow"));

    ctx.cli()
        .args(["delete", "workflow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted item 'workflow'"));

    ctx.cli().arg("list").assert().success().stdout(predicate::str::contains("(none)"));
}
