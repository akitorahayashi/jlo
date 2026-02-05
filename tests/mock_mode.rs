//! Integration tests for mock mode execution.
//!
//! Note: Mock mode creates real branches and PRs, so these tests focus on:
//! 1. CLI argument validation
//! 2. Error handling for missing prerequisites
//! 3. Verifying mock conflicts with dry-run (they're mutually exclusive)

mod common;

use common::TestContext;
use predicates::prelude::*;

/// Helper to initialize scaffold in a test context
fn setup_scaffold(ctx: &TestContext) {
    ctx.cli().args(["init", "scaffold"]).assert().success();
}

#[test]
fn mock_requires_gh_token() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    // --mock requires GH_TOKEN environment variable
    ctx.cli()
        .args(["run", "narrator", "--mock"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("GH_TOKEN"));
}

#[test]
fn mock_conflicts_with_dry_run() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    // --mock and --dry-run are mutually exclusive
    ctx.cli()
        .args(["run", "narrator", "--mock", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
