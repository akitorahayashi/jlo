use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn update_cli_conflicts_with_prompt_preview() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["update", "--cli", "--prompt-preview"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
