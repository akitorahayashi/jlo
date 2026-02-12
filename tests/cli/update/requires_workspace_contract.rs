use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn update_requires_initialized_workspace() {
    let ctx = TestContext::new();

    ctx.cli().args(["update"]).assert().failure().stderr(predicate::str::contains("No .jlo/"));
}
