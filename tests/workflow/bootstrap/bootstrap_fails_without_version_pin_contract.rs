use crate::harness::TestContext;
use predicates::prelude::*;
use std::fs;

#[test]
fn bootstrap_fails_without_control_plane_version_pin() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    let version_file = ctx.jlo_path().join(".jlo-version");
    assert!(version_file.exists(), "precondition: .jlo-version should exist after init");
    fs::remove_file(&version_file).expect("remove .jlo-version");

    ctx.cli()
        .args(["workflow", "bootstrap", "managed-files"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("jlo-version"));
}
