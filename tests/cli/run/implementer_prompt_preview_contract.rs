use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn run_implementer_prompt_preview_renders_session_plan() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    let requirement_dir = ctx.work_dir().join(".jules/exchange/requirements");
    std::fs::create_dir_all(&requirement_dir).expect("create requirements dir");
    std::fs::write(
        requirement_dir.join("test_requirement.yml"),
        "fingerprint: test_requirement\nid: test_requirement\ntitle: Test Requirement\nlabel: bugs\nstatus: open\n",
    )
    .expect("write requirement");

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "run",
            "implementer",
            ".jules/exchange/requirements/test_requirement.yml",
            "--prompt-preview",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Implementer"))
        .stdout(predicate::str::contains("Would execute 1 session"));
}
