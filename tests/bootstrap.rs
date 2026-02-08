//! Integration tests for the bootstrap projection engine.
//!
//! Covers:
//! - Hard precondition failures (missing `.jlo/`, missing `.jlo-version`)
//! - Deletion propagation for workstreams and roles
//! - Idempotent no-op on unchanged inputs

mod common;

use common::TestContext;
use predicates::prelude::*;
use std::fs;

/// Initialize a full workspace (`.jlo/` + `.jules/` + workflows).
fn init_workspace(ctx: &TestContext) {
    ctx.cli().args(["init", "--remote"]).assert().success();
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
// Deletion propagation — workstreams
// ---------------------------------------------------------------------------

#[test]
fn bootstrap_deletes_workstream_absent_from_control_plane() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    // Verify "generic" workstream exists in both planes after init.
    let jlo_generic = ctx.jlo_path().join("workstreams/generic");
    let jules_generic = ctx.jules_path().join("workstreams/generic");
    assert!(jlo_generic.exists(), "precondition: .jlo/ should have generic workstream");
    assert!(jules_generic.exists(), "precondition: .jules/ should have generic workstream");

    // Plant an extra workstream only in .jules/ (simulating stale projection).
    let jules_stale = ctx.jules_path().join("workstreams/stale-ws");
    fs::create_dir_all(&jules_stale).expect("create stale workstream dir");
    fs::write(jules_stale.join("scheduled.toml"), "# stale").expect("write stale file");
    assert!(jules_stale.exists());

    // Bootstrap should remove stale-ws from .jules/ since it is absent from .jlo/.
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    assert!(
        !jules_stale.exists(),
        "stale workstream should be deleted from .jules/ after bootstrap"
    );
    // "generic" should survive (it exists in .jlo/).
    assert!(jules_generic.exists(), "generic workstream should survive bootstrap");
}

#[test]
fn bootstrap_deletes_workstream_removed_from_control_plane() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    let jlo_generic = ctx.jlo_path().join("workstreams/generic");
    let jules_generic = ctx.jules_path().join("workstreams/generic");
    assert!(jlo_generic.exists());
    assert!(jules_generic.exists());

    // Remove "generic" from the control plane.
    fs::remove_dir_all(&jlo_generic).expect("remove generic from .jlo/");

    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    assert!(!jules_generic.exists(), "workstream removed from .jlo/ must be pruned from .jules/");
}

// ---------------------------------------------------------------------------
// Deletion propagation — roles
// ---------------------------------------------------------------------------

#[test]
fn bootstrap_deletes_role_absent_from_control_plane() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    // Plant a stale role in a multi-role layer (observers) only in .jules/.
    let jules_stale_role = ctx.jules_path().join("roles/observers/roles/ghost-role");
    fs::create_dir_all(&jules_stale_role).expect("create stale role dir");
    fs::write(jules_stale_role.join("role.yml"), "# ghost").expect("write ghost role.yml");
    assert!(jules_stale_role.exists());

    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    assert!(
        !jules_stale_role.exists(),
        "stale role should be deleted from .jules/ after bootstrap"
    );

    // Built-in roles should survive because scaffold materializes them.
    let taxonomy = ctx.jules_path().join("roles/observers/roles/taxonomy/role.yml");
    assert!(taxonomy.exists(), "built-in taxonomy role should survive bootstrap");
}

#[test]
fn bootstrap_deletes_role_removed_from_control_plane() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    // Create a custom role via the CLI so it exists in both .jlo/ and .jules/.
    ctx.cli().args(["create", "role", "observers", "custom-obs"]).assert().success();

    let jlo_custom = ctx.jlo_path().join("roles/observers/roles/custom-obs");
    let jules_custom = ctx.jules_path().join("roles/observers/roles/custom-obs");
    assert!(jlo_custom.exists(), "precondition: custom role in .jlo/");

    // Bootstrap to project it into .jules/.
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();
    assert!(jules_custom.exists(), "custom role should be projected into .jules/");

    // Now remove from .jlo/ and re-bootstrap.
    fs::remove_dir_all(&jlo_custom).expect("remove custom role from .jlo/");
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    assert!(!jules_custom.exists(), "role removed from .jlo/ must be pruned from .jules/");
}

// ---------------------------------------------------------------------------
// Idempotent no-op
// ---------------------------------------------------------------------------

#[test]
fn bootstrap_idempotent_on_unchanged_inputs() {
    let ctx = TestContext::new();
    init_workspace(&ctx);

    // First bootstrap.
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Snapshot .jules/ state.
    let snapshot_before = collect_tree_snapshot(&ctx.jules_path());

    // Second bootstrap with identical inputs.
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    let snapshot_after = collect_tree_snapshot(&ctx.jules_path());

    assert_eq!(
        snapshot_before, snapshot_after,
        "re-running bootstrap with unchanged inputs should produce identical .jules/ state"
    );
}

/// Collect a sorted list of (relative_path, content) pairs for all files under `dir`.
fn collect_tree_snapshot(dir: &std::path::Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    collect_tree_recursive(dir, dir, &mut result);
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

fn collect_tree_recursive(
    base: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<(String, String)>,
) {
    if !dir.exists() {
        return;
    }
    let mut entries: Vec<_> = fs::read_dir(dir).expect("read_dir").filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_tree_recursive(base, &path, out);
        } else if path.is_file() {
            let rel = path.strip_prefix(base).unwrap().to_string_lossy().to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            out.push((rel, content));
        }
    }
}
