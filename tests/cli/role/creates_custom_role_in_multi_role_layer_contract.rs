use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn role_create_writes_role_to_control_plane() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli()
        .args(["role", "create", "observers", "security"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created new"));

    let role_path = ctx.jlo_path().join("roles/observers/security/role.yml");
    assert!(role_path.exists(), "Custom role should exist in .jlo/ control plane");
}
