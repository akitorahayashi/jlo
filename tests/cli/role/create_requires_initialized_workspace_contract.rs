use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn role_create_requires_initialized_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["role", "create", "observers", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("jlo init"));
}
