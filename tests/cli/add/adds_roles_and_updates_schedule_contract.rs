use crate::harness::TestContext;
use crate::harness::scheduled_roles::read_scheduled_role_names;
use predicates::prelude::*;

#[test]
fn add_registers_multiple_roles_and_updates_schedule() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli()
        .args(["add", "observers", "rustacean", "gopher"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".jlo/config.toml"));

    let rustacean_path = ctx.jlo_path().join("roles/observers/rustacean/role.yml");
    assert!(
        rustacean_path.exists(),
        "Built-in role should be materialized under .jlo/roles by add"
    );

    let gopher_path = ctx.jlo_path().join("roles/observers/gopher/role.yml");
    assert!(gopher_path.exists(), "Built-in role should be materialized under .jlo/roles by add");

    let roles = read_scheduled_role_names(ctx.work_dir(), "observers");
    assert!(roles.contains(&"rustacean".to_string()));
    assert!(roles.contains(&"gopher".to_string()));
}
