use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn role_create_rejects_path_traversal_like_role_input() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli()
        .args(["role", "create", "observers", "../../tmp/escape"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid role identifier"));
}
