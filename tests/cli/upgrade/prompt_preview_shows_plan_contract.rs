use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn upgrade_prompt_preview_prints_plan() {
    let ctx = TestContext::new();

    ctx.init_remote();

    // Simulate an older version to trigger upgrade logic.
    let version_file = ctx.jlo_path().join(".jlo-version");
    std::fs::write(&version_file, "0.0.0").expect("write version");

    ctx.cli()
        .args(["upgrade", "--prompt-preview"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview"));
}
