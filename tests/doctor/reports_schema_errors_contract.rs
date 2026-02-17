use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn doctor_reports_schema_errors_with_nonzero_exit_code() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    let event_dir = ctx.work_dir().join(".jules/exchange/events/pending");
    std::fs::create_dir_all(&event_dir).unwrap();
    let event_path = event_dir.join("bad-event.yml");
    std::fs::write(
        &event_path,
        "schema_version: 1\nid: abc123\nrequirement_id: \"\"\ncreated_at: 2026-01-01\nauthor_role: tester\nconfidence: low\ntitle: Bad event\nstatement: too short\nevidence: []\n",
    )
    .unwrap();

    ctx.cli()
        .args(["doctor"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("evidence must have entries"));
}
