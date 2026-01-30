mod common;

use common::TestContext;
use serial_test::serial;

#[test]
#[serial]
fn init_creates_workspace_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jlo::init().expect("init should succeed");
    });

    ctx.assert_jules_exists();
    ctx.assert_layer_structure_exists();
}

#[test]
#[serial]
fn template_creates_role_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jlo::init().expect("init should succeed");
        let path = jlo::template(Some("observers"), Some("my-role"), None)
            .expect("template should succeed");
        assert_eq!(path, "observers/my-role");
    });

    ctx.assert_role_in_layer_exists("observers", "my-role");
}
