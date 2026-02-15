use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn create_requires_initialized_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["create", "observers", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("jlo init"));
}
