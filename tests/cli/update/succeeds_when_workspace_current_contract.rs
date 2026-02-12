use crate::harness::TestContext;

#[test]
fn update_succeeds_on_current_workspace() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli().args(["update"]).assert().success();
}
