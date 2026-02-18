use crate::harness::TestContext;

#[test]
fn role_short_aliases_execute_commands() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli().args(["r", "a", "observers", "pythonista"]).assert().success();
    ctx.cli().args(["r", "ad", "observers", "gopher"]).assert().success();

    ctx.cli().args(["r", "c", "observers", "alias-c"]).assert().success();
    ctx.cli().args(["r", "d", "observers", "alias-c"]).assert().success();

    ctx.cli().args(["r", "cr", "observers", "alias-cr"]).assert().success();
    ctx.cli().args(["r", "dl", "observers", "alias-cr"]).assert().success();
}
