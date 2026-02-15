use crate::harness::TestContext;
use crate::harness::scheduled_roles::read_scheduled_role_names;
use predicates::prelude::*;

#[test]
fn add_registers_role_and_updates_schedule() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli()
        .args(["add", "observers", "pythonista"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".jlo/config.toml"));

    let role_path = ctx.jlo_path().join("roles/observers/pythonista/role.yml");
    assert!(
        !role_path.exists(),
        "Built-in role should not be materialized under .jlo/roles by add"
    );

    let roles = read_scheduled_role_names(ctx.work_dir(), "observers");
    assert!(roles.contains(&"pythonista".to_string()));
}
