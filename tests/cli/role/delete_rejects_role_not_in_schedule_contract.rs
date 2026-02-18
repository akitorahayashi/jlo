use crate::harness::TestContext;
use predicates::prelude::*;
use std::fs;

#[test]
fn role_delete_rejects_role_missing_from_schedule() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let role_dir = ctx.jlo_path().join("roles").join("observers").join("orphan");
    fs::create_dir_all(&role_dir).expect("create orphan role dir");
    fs::write(role_dir.join("role.yml"), "role: orphan\nlayer: observers\n")
        .expect("write orphan role.yml");

    ctx.cli()
        .args(["role", "delete", "observers", "orphan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in config"));
}
