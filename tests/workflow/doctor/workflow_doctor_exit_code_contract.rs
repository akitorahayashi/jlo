use crate::harness::TestContext;
use predicates::prelude::*;
use std::fs;

#[test]
fn workflow_doctor_succeeds_on_clean_workspace() {
    let ctx = TestContext::new();
    ctx.init_remote_and_bootstrap();

    ctx.cli()
        .args(["workflow", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));
}

#[test]
fn workflow_doctor_fails_when_checks_fail() {
    let ctx = TestContext::new();
    ctx.init_remote_and_bootstrap();

    let decided_dir = ctx.jules_path().join("exchange/events/decided");
    fs::create_dir_all(&decided_dir).expect("create decided dir");
    fs::write(
        decided_dir.join("bad.yml"),
        r#"schema_version: 1
id: abc123
requirement_id: req001
created_at: "2026-02-01"
author_role: taxonomy
confidence: low
title: "Broken event"
statement: "This statement is long enough for schema checks."
evidence:
  - path: src/main.rs
    note: "missing loc"
"#,
    )
    .expect("write bad event");

    ctx.cli()
        .args(["workflow", "doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stderr(predicate::str::contains("evidence[0].loc is required"));
}
