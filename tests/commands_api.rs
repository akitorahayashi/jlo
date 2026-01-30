mod common;

use common::TestContext;
use serial_test::serial;

#[test]
#[serial]
fn init_creates_workspace_via_cli() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("init")
        .assert()
        .success();

    ctx.assert_jules_exists();
    ctx.assert_layer_structure_exists();
}

#[test]
#[serial]
fn template_creates_role_via_cli() {
    let ctx = TestContext::new();

    // Init first
    ctx.cli()
        .arg("init")
        .assert()
        .success();

    // Create template
    ctx.cli()
        .args(["template", "--layer", "observers", "--name", "my-role"])
        .assert()
        .success();

    ctx.assert_role_in_layer_exists("observers", "my-role");
}
