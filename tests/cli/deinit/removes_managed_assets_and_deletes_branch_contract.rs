use crate::harness::TestContext;
use crate::harness::git_repository;
use predicates::prelude::*;
use std::fs;

#[test]
fn deinit_removes_managed_assets_and_deletes_local_jules_branch() {
    let ctx = TestContext::new();

    let seed_file = ctx.work_dir().join("seed.txt");
    fs::write(&seed_file, "seed").expect("write seed");

    git_repository::configure_user(ctx.work_dir());
    git_repository::commit_all(ctx.work_dir(), "seed");

    ctx.init_remote();

    // Create a 'jules' branch so deinit can delete it, then return to control branch.
    ctx.git_checkout_branch("jules", true);
    let output = std::process::Command::new("git")
        .args(["checkout", "-"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git checkout - failed");
    assert!(output.status.success(), "switch back to control branch failed");

    ctx.cli()
        .args(["deinit"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed .jlo/ control plane"))
        .stdout(predicate::str::contains("Deleted local 'jules' branch"));

    assert!(!ctx.work_dir().join(".jlo").exists(), ".jlo/ should be removed after deinit");
    assert!(
        !ctx.work_dir().join(".github/workflows/jules-workflows.yml").exists(),
        "workflow kit file should be removed"
    );
    assert!(
        !ctx.work_dir().join(".github/actions/install-jlo/action.yml").exists(),
        "workflow action should be removed"
    );
}
