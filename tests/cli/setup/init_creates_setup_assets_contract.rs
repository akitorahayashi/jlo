use crate::harness::TestContext;

#[test]
fn init_creates_setup_assets_in_control_plane() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    assert!(ctx.work_dir().join(".jlo/setup").exists());
    assert!(ctx.work_dir().join(".jlo/setup/tools.yml").exists());
    assert!(ctx.work_dir().join(".jlo/setup/.gitignore").exists());
}
