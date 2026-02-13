use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn setup_gen_generates_install_script_vars_and_secrets_in_control_plane() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    // Write tools config in .jlo.
    let tools_yml = ctx.work_dir().join(".jlo/setup/tools.yml");
    std::fs::write(&tools_yml, "tools:\n  - just\n").expect("write tools.yml");

    ctx.cli()
        .args(["setup", "gen"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated install.sh"));

    assert!(ctx.work_dir().join(".jlo/setup/install.sh").exists());
    assert!(ctx.work_dir().join(".jlo/setup/vars.toml").exists());
    assert!(ctx.work_dir().join(".jlo/setup/secrets.toml").exists());
}
