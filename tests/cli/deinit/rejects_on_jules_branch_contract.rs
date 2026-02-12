use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn deinit_rejects_when_on_jules_branch() {
    let ctx = TestContext::new();

    ctx.git_checkout_branch("jules", true);

    ctx.cli()
        .args(["deinit"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cannot deinit while on branch"));
}
