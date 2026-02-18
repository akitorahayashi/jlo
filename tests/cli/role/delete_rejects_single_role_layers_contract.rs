use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn role_delete_rejects_single_role_layers() {
    let ctx = TestContext::new();

    ctx.init_remote();

    for layer in ["narrator", "decider", "planner", "implementer", "integrator"] {
        ctx.cli()
            .args(["role", "delete", layer, "custom"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("single-role"));
    }
}
