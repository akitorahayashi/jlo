//! Integration tests for mock mode execution.
//!
//! Note: Mock mode creates real branches and PRs, so these tests focus on:
//! 1. CLI argument validation
//! 2. Error handling for missing prerequisites
//! 3. Verifying mock conflicts with prompt-preview (they're mutually exclusive)

mod common;

use common::TestContext;
use predicates::prelude::*;

/// Helper to initialize scaffold in a test context
fn setup_scaffold(ctx: &TestContext) {
    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();
}

#[test]
fn mock_requires_gh_token() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    // Add a dummy origin remote to avoid git fetch errors
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/test/test.git"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("Failed to add origin remote");

    // --mock requires GH_TOKEN environment variable
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

    // --mock and --prompt-preview are mutually exclusive
    ctx.cli()
        .args(["run", "narrator", "--mock", "--prompt-preview"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
