use crate::harness::TestContext;

#[test]
fn bootstrap_managed_files_subcommand_runs_independently() {
    let ctx = TestContext::new();
    ctx.init_remote();

    ctx.cli().args(["workflow", "bootstrap", "managed-files"]).assert().success();
    assert!(
        ctx.jules_path().join(".jlo-version").exists(),
        "managed-files subcommand should stamp .jules version file"
    );
}
