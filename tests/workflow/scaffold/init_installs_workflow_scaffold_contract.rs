use crate::harness::TestContext;
use predicates::prelude::*;
use std::fs;

#[test]
fn init_installs_remote_workflow_scaffold() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    assert!(root.join(".github/workflows/jules-scheduled-workflows.yml").exists());
    assert!(root.join(".github/workflows/jules-implementer-pr.yml").exists());
    assert!(root.join(".github/workflows/jules-automerge.yml").exists());
    assert!(root.join(".github/workflows/jules-integrator.yml").exists());
    assert!(root.join(".github/actions/install-jlo/action.yml").exists());
    assert!(
        !root.join(".github/actions/wait/action.yml").exists(),
        "Workflow scaffold should not include legacy wait action"
    );
    assert!(
        !root.join(".github/workflows/jules-workflows/components").exists(),
        "Workflow scaffold should not include workflow template components"
    );
    assert!(
        !root.join(".github/scripts").exists(),
        "Workflow scaffold should not include .github/scripts"
    );

    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml")).unwrap();
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(workflow.contains("Run observers"), "Should run observers");
    assert!(workflow.contains("Run decider"), "Should run decider");
    assert!(workflow.contains("Run planner"), "Should run planner");
    assert!(workflow.contains("Run implementer"), "Should run implementer");
    assert!(workflow.contains("run-innovators:"), "Should include integrated run-innovators job");
    assert!(
        workflow.contains("Publish proposals"),
        "Scheduled workflow should include innovators publish step"
    );
    assert!(
        !workflow.contains(".github/scripts/"),
        "Workflow should not reference .github/scripts/"
    );
    assert!(
        !workflow.contains("{% include"),
        "Generated workflow should not contain template include directives"
    );
}

#[test]
fn init_installs_self_hosted_workflow_scaffold() {
    let ctx = TestContext::new();

    ctx.init_self_hosted();

    let root = ctx.work_dir();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml")).unwrap();

    assert!(workflow.contains("runs-on: self-hosted"), "Should use self-hosted runner");
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(workflow.contains("Run observers"), "Should run observers");
}

#[test]
fn init_requires_runner_mode() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().failure().stderr(predicate::str::contains("required"));
}
