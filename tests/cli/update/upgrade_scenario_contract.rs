use crate::harness::TestContext;
use std::fs;

#[test]
fn update_advances_version_and_preserves_config() {
    let ctx = TestContext::new();

    // 1. Initialize remote (fresh)
    ctx.init_remote();

    // 2. Simulate older state
    let jlo_version_path = ctx.jlo_path().join(".jlo-version");
    fs::write(&jlo_version_path, "0.0.0").expect("Failed to downgrade version");

    // 3. Run update (advances version)
    ctx.cli().args(["update"]).assert().success();

    // 4. Verify version updated
    let version = fs::read_to_string(&jlo_version_path).expect("Failed to read version file");
    assert_ne!(version.trim(), "0.0.0", "Version should have been updated from 0.0.0");

    // 5. Verify no regression in config (it should still exist)
    assert!(ctx.jlo_path().join("config.toml").exists(), "config.toml should still exist");
}
