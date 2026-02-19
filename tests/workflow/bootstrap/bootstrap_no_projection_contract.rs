use crate::harness::TestContext;
use std::fs;

#[test]
fn bootstrap_does_not_project_control_plane_roles() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    // Create a custom role in .jlo.
    let custom_role_jlo = ctx.jlo_path().join("roles/observers/custom-obs");
    fs::create_dir_all(&custom_role_jlo).expect("create custom role dir");
    fs::write(custom_role_jlo.join("role.yml"), "role: custom").expect("write role.yml");

    ctx.cli().args(["workflow", "bootstrap", "managed-files"]).assert().success();

    // Verify it is NOT in .jules/.
    assert!(
        !ctx.jules_path().join("roles/observers/custom-obs").exists(),
        "Custom role from .jlo/ should NOT be projected to .jules/"
    );

    // But `.jules/` structure should exist (e.g. README.md).
    assert!(ctx.jules_path().join("README.md").exists());
}

#[test]
fn bootstrap_does_not_project_unknown_control_plane_directories() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    let custom_dir_jlo = ctx.jlo_path().join("custom-project");
    fs::create_dir_all(&custom_dir_jlo).expect("create custom dir");
    fs::write(custom_dir_jlo.join("data.toml"), "").expect("write data.toml");

    ctx.cli().args(["workflow", "bootstrap", "managed-files"]).assert().success();

    assert!(
        !ctx.jules_path().join("custom-project").exists(),
        "Custom directory from .jlo/ should NOT be projected to .jules/"
    );
}
