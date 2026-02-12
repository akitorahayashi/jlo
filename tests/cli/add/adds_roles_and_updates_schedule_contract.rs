use crate::harness::TestContext;
use crate::harness::scheduled_roles::read_scheduled_role_names;
use predicates::prelude::*;

#[test]
fn add_installs_multiple_roles_and_updates_schedule() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli()
        .args(["add", "observers", "rustacean", "gopher"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".jlo/roles/observers/rustacean/"))
        .stdout(predicate::str::contains(".jlo/roles/observers/gopher/"));

    let rustacean_path = ctx.jlo_path().join("roles/observers/rustacean/role.yml");
    assert!(rustacean_path.exists(), "First added role should exist in .jlo/");

    let gopher_path = ctx.jlo_path().join("roles/observers/gopher/role.yml");
    assert!(gopher_path.exists(), "Second added role should exist in .jlo/");

    let roles = read_scheduled_role_names(ctx.work_dir(), "observers");
    assert!(roles.contains(&"rustacean".to_string()));
    assert!(roles.contains(&"gopher".to_string()));
}
