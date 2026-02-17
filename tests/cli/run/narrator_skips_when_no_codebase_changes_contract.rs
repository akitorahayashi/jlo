use crate::harness::TestContext;
use crate::harness::git_repository;
use predicates::prelude::*;

#[test]
fn run_narrator_dispatches_even_without_codebase_changes() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    git_repository::configure_user(ctx.work_dir());
    git_repository::commit_all(ctx.work_dir(), "initial");

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args(["run", "narrator", "--prompt-preview"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Narrator"));
}
