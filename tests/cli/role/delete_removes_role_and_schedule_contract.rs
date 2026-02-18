use crate::harness::TestContext;
use crate::harness::scheduled_roles::read_scheduled_role_names;
use predicates::prelude::*;

#[test]
fn role_delete_removes_role_directory_and_schedule_entry() {
    let ctx = TestContext::new();

    ctx.init_remote();

    ctx.cli().args(["role", "create", "observers", "to-delete"]).assert().success();

    ctx.cli()
        .args(["role", "delete", "observers", "to-delete"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    let role_path = ctx.jlo_path().join("roles/observers/to-delete/role.yml");
    assert!(!role_path.exists(), "Deleted role should not exist in .jlo/roles");

    let roles = read_scheduled_role_names(ctx.work_dir(), "observers");
    assert!(
        !roles.contains(&"to-delete".to_string()),
        "Deleted role should be removed from schedule"
    );
}
