use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn run_implementer_requires_requirement_argument() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    ctx.cli()
        .args(["run", "implementer"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn run_planner_requires_requirement_argument() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    ctx.cli()
        .args(["run", "planner"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}
