use crate::harness::TestContext;
use std::fs;

#[test]
fn init_remote_creates_setup_assets_and_generated_artifacts_in_control_plane() {
    let ctx = TestContext::new();

    ctx.init_remote();

    assert!(ctx.work_dir().join(".jlo/setup").exists());
    assert!(ctx.work_dir().join(".jlo/setup/tools.yml").exists());
    assert!(ctx.work_dir().join(".jlo/setup/.gitignore").exists());
    assert_eq!(
        fs::read_to_string(ctx.work_dir().join(".jlo/setup/.gitignore")).unwrap(),
        "# Ignore secret environment configuration only\nsecrets.toml\n"
    );
    assert!(ctx.work_dir().join(".jlo/setup/install.sh").exists());
    assert!(ctx.work_dir().join(".jlo/setup/vars.toml").exists());
    assert!(ctx.work_dir().join(".jlo/setup/secrets.toml").exists());
}

#[test]
fn init_self_hosted_creates_setup_assets_and_generated_artifacts_in_control_plane() {
    let ctx = TestContext::new();

    ctx.init_self_hosted();

    assert!(ctx.work_dir().join(".jlo/setup").exists());
    assert!(ctx.work_dir().join(".jlo/setup/tools.yml").exists());
    assert!(ctx.work_dir().join(".jlo/setup/.gitignore").exists());
    assert_eq!(
        fs::read_to_string(ctx.work_dir().join(".jlo/setup/.gitignore")).unwrap(),
        "# Ignore secret environment configuration only\nsecrets.toml\n"
    );
    assert!(ctx.work_dir().join(".jlo/setup/install.sh").exists());
    assert!(ctx.work_dir().join(".jlo/setup/vars.toml").exists());
    assert!(ctx.work_dir().join(".jlo/setup/secrets.toml").exists());
}
