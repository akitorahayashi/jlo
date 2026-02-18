use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn upgrade_requires_initialized_workspace() {
    let ctx = TestContext::new();

    ctx.cli().args(["upgrade"]).assert().failure().stderr(predicate::str::contains("No .jlo/"));
}
