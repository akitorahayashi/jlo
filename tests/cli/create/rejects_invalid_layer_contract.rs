use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn create_rejects_invalid_layer_name() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli()
        .args(["create", "invalid", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid layer"));
}
