use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn setup_gen_requires_setup_initialized() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "gen"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Setup not initialized"));
}
