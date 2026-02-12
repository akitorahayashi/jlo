use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn init_rejects_when_already_initialized() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli()
        .args(["init", "--remote"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}
