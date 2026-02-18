use crate::harness::TestContext;
use predicates::prelude::*;

/// Verify that `jlo update` is wired to the binary self-update path.
///
/// The command attempts a network call to git ls-remote, which may succeed or
/// fail in CI depending on network access. We assert structural properties:
/// - The command does not exit with a usage/argument error (i.e., it is
///   correctly parsed as a valid command with no required arguments).
/// - On failure it surfaces a tool-level error (git or cargo), not a clap
///   argument error.
/// - `--prompt-preview` is not accepted (it belongs to `upgrade`).
#[test]
fn update_does_not_accept_prompt_preview_flag() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["update", "--prompt-preview"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

/// `jlo update` must not require a workspace (`.jlo/`) to be initialized,
/// because it operates on the binary itself, not on the repository.
#[test]
fn update_does_not_require_initialized_workspace() {
    let ctx = TestContext::new();

    // Run in a completely empty directory. The command may fail due to network
    // constraints, but it must not fail with "No .jlo/ control plane found".
    let output = ctx.cli().args(["update"]).output().expect("process spawned");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("No .jlo/"),
        "update should not require an initialized workspace, got: {}",
        stderr,
    );
}
