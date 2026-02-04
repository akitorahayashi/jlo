mod common;

use common::TestContext;
use predicates::prelude::*;
use std::fs;

#[test]
fn init_workflows_installs_remote_kit() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["init", "workflows", "--remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed workflow kit"));

    let root = ctx.work_dir();
    assert!(root.join(".github/workflows/jules-workflows.yml").exists());
    assert!(root.join(".github/actions/install-jlo/action.yml").exists());
    assert!(root.join(".github/scripts/jules-generate-workstream-matrix.sh").exists());
    assert!(!root.join(".jules").exists());
}

#[test]
fn init_workflows_installs_self_hosted_kit() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["init", "workflows", "--self-hosted"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed workflow kit"));

    let root = ctx.work_dir();
    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    assert!(workflow_path.exists());

    let content = fs::read_to_string(&workflow_path).unwrap();
    assert!(content.contains("runs-on: self-hosted"));

    assert!(!root.join(".github/scripts/jules-run-observers-sequential.sh").exists());
}

#[test]
fn init_workflows_requires_runner_mode() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["init", "workflows"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn init_workflows_fails_on_collision_without_overwrite() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(&workflow_path, "collision").unwrap();

    ctx.cli()
        .args(["init", "workflows", "--remote"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Workflow kit install aborted"))
        .stderr(predicate::str::contains("--overwrite"));
}

#[test]
fn init_workflows_overwrite_respects_unrelated_files() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    let kit_workflow = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(kit_workflow.parent().unwrap()).unwrap();
    fs::write(&kit_workflow, "name: Old Workflow\non:\n  workflow_dispatch:\n").unwrap();

    let unrelated_workflow = root.join(".github/workflows/unrelated.yml");
    fs::write(&unrelated_workflow, "keep me").unwrap();

    let kit_action = root.join(".github/actions/install-jlo/action.yml");
    fs::create_dir_all(kit_action.parent().unwrap()).unwrap();
    fs::write(&kit_action, "old action").unwrap();

    let unrelated_action = root.join(".github/actions/custom/action.yml");
    fs::create_dir_all(unrelated_action.parent().unwrap()).unwrap();
    fs::write(&unrelated_action, "custom action").unwrap();

    ctx.cli().args(["init", "workflows", "--remote", "--overwrite"]).assert().success();

    let updated_workflow = fs::read_to_string(&kit_workflow).unwrap();
    assert!(updated_workflow.contains("Jules Workflows"));

    let updated_action = fs::read_to_string(&kit_action).unwrap();
    assert!(updated_action.contains("Install jlo"));

    let unrelated_content = fs::read_to_string(&unrelated_workflow).unwrap();
    assert_eq!(unrelated_content, "keep me");

    let unrelated_action_content = fs::read_to_string(&unrelated_action).unwrap();
    assert_eq!(unrelated_action_content, "custom action");
}

#[test]
fn init_workflows_overwrite_preserves_schedule() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(
        &workflow_path,
        "name: Existing Workflow\non:\n  schedule:\n    - cron: '15 3 * * *'\n  workflow_dispatch:\n",
    )
    .unwrap();

    ctx.cli().args(["init", "workflows", "--remote", "--overwrite"]).assert().success();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    assert!(updated_workflow.contains("15 3 * * *"));
    assert!(!updated_workflow.contains("0 20 * * *"));
}

#[test]
fn init_workflows_overwrite_fails_on_invalid_schedule() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(
        &workflow_path,
        "name: Existing Workflow\non:\n  schedule: daily\n  workflow_dispatch:\n",
    )
    .unwrap();

    ctx.cli()
        .args(["init", "workflows", "--remote", "--overwrite"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("schedule").and(predicate::str::contains("sequence")));
}
