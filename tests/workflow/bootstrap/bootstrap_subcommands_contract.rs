use crate::harness::TestContext;

#[test]
fn bootstrap_subcommands_can_run_independently() {
    let ctx = TestContext::new();
    ctx.init_remote();

    ctx.cli().args(["workflow", "bootstrap", "managed-files"]).assert().success();
    assert!(
        ctx.jules_path().join(".jlo-version").exists(),
        "managed-files subcommand should stamp .jules version file"
    );

    ctx.cli().args(["workflow", "bootstrap", "workstations"]).assert().success();
    assert!(
        ctx.jules_path().join("workstations").exists(),
        "workstations subcommand should reconcile workstation directory"
    );
}
