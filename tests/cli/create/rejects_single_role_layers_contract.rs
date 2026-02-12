use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn create_rejects_single_role_layers() {
    let ctx = TestContext::new();

    ctx.init_remote();

    for layer in ["narrator", "planner", "implementer"] {
        ctx.cli()
            .args(["create", layer, "custom"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("single-role"));
    }
}
