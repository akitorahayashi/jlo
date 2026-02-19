use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn run_planner_prompt_preview_renders_session_plan() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    // Planner runs on worker branch per branch contract.
    ctx.git_checkout_branch("jules", true);

    let requirement_dir = ctx.work_dir().join(".jules/exchange/requirements");
    std::fs::create_dir_all(&requirement_dir).expect("create requirements dir");
    std::fs::write(
        requirement_dir.join("test_requirement.yml"),
        "fingerprint: test_requirement\nid: test_requirement\ntitle: Test Requirement\nstatus: open\nimplementation_ready: false\nplanner_request_reason: \"Needs planner elaboration\"\n",
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
