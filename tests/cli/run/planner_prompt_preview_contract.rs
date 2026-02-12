use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn run_planner_prompt_preview_renders_session_plan() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    let requirement_dir = ctx.work_dir().join(".jules/exchange/requirements");
    std::fs::create_dir_all(&requirement_dir).expect("create requirements dir");
    std::fs::write(
        requirement_dir.join("test_requirement.yml"),
        "fingerprint: test_requirement\nid: test_requirement\ntitle: Test Requirement\nstatus: open\nrequires_deep_analysis: true\n",
    )
    .expect("write requirement");

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "run",
            "planner",
            "--requirement",
            ".jules/exchange/requirements/test_requirement.yml",
            "--prompt-preview",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Planner"))
        .stdout(predicate::str::contains("Would execute 1 session"));
}
