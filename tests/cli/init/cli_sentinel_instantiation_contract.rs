use crate::harness::TestContext;

#[test]
fn init_instantiates_cli_sentinel() {
    let ctx = TestContext::new();

    ctx.init_remote();

    // Check scheduled.toml content
    let scheduled_path = ctx.jlo_path().join("scheduled.toml");
    let content = std::fs::read_to_string(scheduled_path).expect("read scheduled.toml");
    assert!(content.contains("name = \"cli_sentinel\""));
    assert!(content.contains("enabled = true"));

    // Check role file exists
    ctx.assert_role_in_layer_exists("observers", "cli_sentinel");
}
