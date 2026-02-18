use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn role_delete_requires_initialized_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["role", "delete", "observers", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("jlo init"));
}
