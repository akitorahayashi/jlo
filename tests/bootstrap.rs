//! Integration tests for the bootstrap projection engine.
//!
//! Covers:
//! - Hard precondition failures (missing `.jlo/`, missing `.jlo-version`)
//! - Framework materialization (scaffold files are created)
//! - Validation that control plane files are NOT projected to `.jules/`

mod common;

use common::TestContext;
use predicates::prelude::*;
use std::fs;

/// Initialize a full workspace (`.jlo/` + workflows) and materialize `.jules/` via bootstrap.
fn init_workspace(ctx: &TestContext) {
    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();
}

// ---------------------------------------------------------------------------
// Hard precondition failures
// ---------------------------------------------------------------------------

#[test]
fn bootstrap_fails_without_jlo_directory() {
    let ctx = TestContext::new();

    // No init — `.jlo/` does not exist.
    ctx.cli()
        .args(["workflow", "bootstrap"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("control plane"));
}

#[test]
fn bootstrap_fails_without_jlo_version_file() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    // Remove the version file from the control plane.
    let version_file = ctx.jlo_path().join(".jlo-version");
    assert!(version_file.exists(), "precondition: .jlo-version should exist after init");
    fs::remove_file(&version_file).expect("remove .jlo-version");

    ctx.cli()
        .args(["workflow", "bootstrap"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("jlo-version"));
}

// ---------------------------------------------------------------------------
// No Projection Verification
// ---------------------------------------------------------------------------

#[test]
fn bootstrap_does_not_project_roles() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    // Create a custom role in .jlo
    let custom_role_jlo = ctx.jlo_path().join("roles/observers/custom-obs");
    fs::create_dir_all(&custom_role_jlo).expect("create custom role dir");
    fs::write(custom_role_jlo.join("role.yml"), "role: custom").expect("write role.yml");

    // Run bootstrap
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Verify it is NOT in .jules/
    let custom_role_jules = ctx.jules_path().join("roles/observers/custom-obs");
    assert!(
        !custom_role_jules.exists(),
        "Custom role from .jlo/ should NOT be projected to .jules/"
    );

    // But .jules/ structure should exist (e.g. README.md)
    assert!(ctx.jules_path().join("README.md").exists());
}

#[test]
fn bootstrap_does_not_project_control_plane_dirs() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    // Create a custom directory in .jlo
    let custom_dir_jlo = ctx.jlo_path().join("custom-project");
    fs::create_dir_all(&custom_dir_jlo).expect("create custom dir");
    fs::write(custom_dir_jlo.join("data.toml"), "").expect("write data.toml");

    // Run bootstrap
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Verify it is NOT in .jules/
    let custom_dir_jules = ctx.jules_path().join("custom-project");
    assert!(
        !custom_dir_jules.exists(),
        "Custom directory from .jlo/ should NOT be projected to .jules/"
    );
}

#[test]
fn bootstrap_does_not_delete_unknown_files_in_jules() {
    // Since we removed "pruning" logic, bootstrap should strictly be additive (or overwrite managed files).
    // It should NOT delete files it doesn't know about, because it's no longer syncing a mirror.
    // Wait, if it generates the scaffold, does it clear the directory?
    // Implementation: `ctx.workspace().create_structure` uses `create_dir_all`. It does NOT clean first.
    // So random files should survive.

    let ctx = TestContext::new();
    init_workspace(&ctx);

    let random_file = ctx.jules_path().join("random.txt");
    fs::write(&random_file, "I survive").expect("write random file");

    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    assert!(random_file.exists(), "Unmanaged file in .jules/ should survive bootstrap");
}
