use crate::harness::{TestContext, git_repository};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn mock_decider_creates_prs() {
    let mut ctx = TestContext::new();
    ctx.setup_fake_gh();

    // Configure user for local repo
    git_repository::configure_user(ctx.work_dir());

    // Setup remote repo
    let remote_dir = TempDir::new().unwrap();
    git_repository::init_bare_repo(remote_dir.path());
    let remote_url = remote_dir.path().to_str().unwrap();

    // Initialize workspace and commit
    ctx.init_remote_and_bootstrap();
    git_repository::commit_all(ctx.work_dir(), "Initial bootstrap");

    // Add remote
    git_repository::add_origin_remote(ctx.work_dir(), remote_url);

    // Push main branch to origin
    let status = std::process::Command::new("git")
        .args(["push", "origin", "master:main"]) // Push local master (if default) to remote main
        .current_dir(ctx.work_dir())
        .status()
        .expect("failed to push main");

    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(ctx.work_dir())
        .output()
        .unwrap();
    let branch = String::from_utf8(output.stdout).unwrap();
    let branch = branch.trim();

    if !status.success() {
        // Maybe push current branch to main
        std::process::Command::new("git")
            .args(["push", "origin", &format!("{}:main", branch)])
            .current_dir(ctx.work_dir())
            .status()
            .expect("failed to push main");
    }

    // Create jules branch and push it
    ctx.git_checkout_branch("jules", true);

    // Add pending events
    let pending_dir = ctx.jules_path().join("exchange/events/pending");
    fs::create_dir_all(&pending_dir).expect("create pending dir");
    fs::write(
        pending_dir.join("mock-e2e-decider-test-event1.yml"),
        "id: event1\nsummary: s1\ncreated_at: '2023-10-27'\nevidence:\n  loc: []\n",
    )
    .expect("write event1");
    fs::write(
        pending_dir.join("mock-e2e-decider-test-event2.yml"),
        "id: event2\nsummary: s2\ncreated_at: '2023-10-27'\nevidence:\n  loc: []\n",
    )
    .expect("write event2");

    git_repository::commit_all(ctx.work_dir(), "Add pending events");

    std::process::Command::new("git")
        .args(["push", "origin", "jules"])
        .current_dir(ctx.work_dir())
        .status()
        .expect("failed to push jules");

    // Run jlo run decider --mock
    ctx.cli()
        .env("JULES_MOCK_TAG", "mock-e2e-decider-test")
        .env("GH_TOKEN", "fake-token")
        .args(["run", "decider", "--mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mock decider: created PR"));
}

#[test]
fn mock_implementer_creates_prs() {
    let mut ctx = TestContext::new();
    ctx.setup_fake_gh();

    // Configure user
    git_repository::configure_user(ctx.work_dir());

    // Setup remote repo
    let remote_dir = TempDir::new().unwrap();
    git_repository::init_bare_repo(remote_dir.path());
    let remote_url = remote_dir.path().to_str().unwrap();

    // Initialize
    ctx.init_remote_and_bootstrap();
    git_repository::commit_all(ctx.work_dir(), "Initial bootstrap");

    // Add remote
    git_repository::add_origin_remote(ctx.work_dir(), remote_url);

    // Push main branch
    let status = std::process::Command::new("git")
        .args(["push", "origin", "master:main"])
        .current_dir(ctx.work_dir())
        .status()
        .expect("failed to push main");

    if !status.success() {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(ctx.work_dir())
            .output()
            .unwrap();
        let branch = String::from_utf8(output.stdout).unwrap();
        let branch = branch.trim();
        std::process::Command::new("git")
            .args(["push", "origin", &format!("{}:main", branch)])
            .current_dir(ctx.work_dir())
            .status()
            .expect("failed to push main");
    }

    // Push jules branch (optional for implementer mock, but good practice)
    ctx.git_checkout_branch("jules", true);
    std::process::Command::new("git")
        .args(["push", "origin", "jules"])
        .current_dir(ctx.work_dir())
        .status()
        .expect("failed to push jules");

    // Checkout main again
    ctx.git_checkout_branch("main", false);

    // Create requirement
    let req_dir = ctx.jules_path().join("exchange/requirements");
    fs::create_dir_all(&req_dir).expect("create req dir");
    let req_path = req_dir.join("req.yml");
    fs::write(&req_path, "id: abc123\nlabel: bugs\ntitle: Fix bug\n").expect("write req");

    git_repository::commit_all(ctx.work_dir(), "Add requirement");

    // Run jlo run implementer --mock
    // Use relative path to avoid /var vs /private/var issues on macOS
    ctx.cli()
        .env("JULES_MOCK_TAG", "mock-e2e-impl-test")
        .env("GH_TOKEN", "fake-token")
        .args(["run", "implementer", ".jules/exchange/requirements/req.yml", "--mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mock implementer: created PR"));
}
