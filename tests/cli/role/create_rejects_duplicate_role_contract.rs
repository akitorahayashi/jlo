use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn role_create_rejects_duplicate_role_id() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli().args(["role", "create", "observers", "my-obs"]).assert().success();

    ctx.cli()
        .args(["role", "create", "observers", "my-obs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}
