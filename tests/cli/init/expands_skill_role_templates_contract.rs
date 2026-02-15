use crate::harness::TestContext;

#[test]
fn init_expands_skill_role_templates_into_control_plane() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let observer_template =
        ctx.jlo_path().join("roles/.agents/skills/create-jlo-observer/role.yml");
    assert!(observer_template.exists(), "observer skill role template should be expanded by init");

    let innovator_template =
        ctx.jlo_path().join("roles/.agents/skills/create-jlo-innovator/role.yml");
    assert!(
        innovator_template.exists(),
        "innovator skill role template should be expanded by init"
    );
}
