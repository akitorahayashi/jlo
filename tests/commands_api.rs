mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn init_creates_workspace_via_cli() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.assert_jules_exists();
    ctx.assert_layer_structure_exists();
}

#[test]
#[serial]
fn create_role_via_cli() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli()
        .args(["create", "observers", "my-role"])
        .assert()
        .success()
        .stdout(predicate::str::contains("observers"));

    // Role should exist in .jlo/ control plane
    let role_path = ctx.jlo_path().join("roles/observers/roles/my-role/role.yml");
    assert!(role_path.exists(), "Role should exist in .jlo/ at {}", role_path.display());
}
