use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn setup_list_shows_available_components() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available components:"))
        .stdout(predicate::str::contains("gh"))
        .stdout(predicate::str::contains("just"));
}

#[test]
fn setup_list_detail_shows_component_details() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list", "--detail", "just"])
        .assert()
        .success()
        .stdout(predicate::str::contains("just:"))
        .stdout(predicate::str::contains("Install Script:"));
}

#[test]
fn setup_list_detail_rejects_unknown_component() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list", "--detail", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
