use crate::harness::TestContext;
use crate::harness::git_repository;
use predicates::prelude::*;

#[test]
fn run_narrator_prompt_preview_shows_target_range() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    git_repository::configure_user(ctx.work_dir());

    // Create first commit (includes both .jules/ and README.md).
    std::fs::write(ctx.work_dir().join("README.md"), "# Test Project\n").expect("write readme");
    git_repository::commit_all(ctx.work_dir(), "initial");

    // Create second commit with codebase changes to have a non-empty range.
    std::fs::write(ctx.work_dir().join("README.md"), "# Test Project\n\nUpdated content.\n")
        .expect("write updated readme");
    git_repository::commit_all(ctx.work_dir(), "update readme");

    // Narrator runs on worker branch per branch contract.
    ctx.git_checkout_branch("jules", true);

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args(["run", "narrator", "--prompt-preview"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Narrator"))
        .stdout(predicate::str::contains("Target Range"));
}
