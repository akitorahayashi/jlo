use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn run_implementer_rejects_missing_requirement_file() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    ctx.cli()
        .args(["run", "implementer", ".jules/exchange/requirements/nonexistent.yml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Requirement file not found"));
}
