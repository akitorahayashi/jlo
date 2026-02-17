use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn bootstrap_fails_without_control_plane() {
    let ctx = TestContext::new();

    // No init — `.jlo/` does not exist.
    ctx.cli()
        .args(["workflow", "bootstrap", "managed-files"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("control plane"));
}
