use crate::harness::TestContext;
use crate::harness::git_repository;
use predicates::prelude::*;

fn setup_scaffold(ctx: &TestContext) {
    ctx.init_remote_and_bootstrap();
    // Narrator runs on worker branch per branch contract.
    ctx.git_checkout_branch("jules", true);
}

#[test]
fn mock_requires_gh_token() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    git_repository::add_origin_remote(ctx.work_dir(), "https://github.com/test/test.git");

    ctx.cli()
        .args(["run", "narrator", "--mock"])
        .env_remove("GH_TOKEN")
        .assert()
        .failure()
        .stderr(predicate::str::contains("GH_TOKEN"));
}

#[test]
fn mock_conflicts_with_prompt_preview() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    ctx.cli()
        .args(["run", "narrator", "--mock", "--prompt-preview"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
