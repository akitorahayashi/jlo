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

    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(workflow.contains("Run observers sequentially"), "Should run observers sequentially");
    assert!(workflow.contains("Run deciders sequentially"), "Should run deciders sequentially");
    assert!(workflow.contains("Run planners sequentially"), "Should run planners sequentially");
    assert!(
        workflow.contains("Run implementers sequentially"),
        "Should run implementers sequentially"
    );
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
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();

    assert!(workflow.contains("runs-on: self-hosted"), "Should use self-hosted runner");
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(workflow.contains("Run observers sequentially"), "Should run observers sequentially");
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
    fs::write(&kit_workflow, "old workflow").unwrap();

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

    // Create an existing workflow with a custom schedule
    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    let existing_workflow = r#"name: Jules Workflows

on:
  schedule:
    - cron: '0 12 * * 1-5'
    - cron: '0 6 * * 0'
  workflow_dispatch: {}

jobs:
  test: {}
"#;
    fs::write(&workflow_path, existing_workflow).unwrap();

    ctx.cli().args(["init", "workflows", "--remote", "--overwrite"]).assert().success();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    // The preserved schedule should contain the custom cron entries
    assert!(
        updated_workflow.contains("0 12 * * 1-5"),
        "Custom weekday schedule should be preserved"
    );
    assert!(updated_workflow.contains("0 6 * * 0"), "Custom weekend schedule should be preserved");
    // The kit content should still be present
    assert!(updated_workflow.contains("Jules"), "Workflow name should be present");
}

#[test]
fn init_workflows_overwrite_fails_on_invalid_schedule() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    // Create an existing workflow with invalid YAML
    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    let invalid_yaml = "name: [invalid\n  yaml: content";
    fs::write(&workflow_path, invalid_yaml).unwrap();

    ctx.cli()
        .args(["init", "workflows", "--remote", "--overwrite"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse"));
}

#[test]
fn init_workflows_overwrite_uses_kit_schedule_when_none_exists() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    // Create an existing workflow without a schedule
    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    let existing_workflow = r#"name: Jules Workflows

on:
  workflow_dispatch: {}

jobs:
  test: {}
"#;
    fs::write(&workflow_path, existing_workflow).unwrap();

    ctx.cli().args(["init", "workflows", "--remote", "--overwrite"]).assert().success();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    // The kit's default schedule should be present
    assert!(
        updated_workflow.contains("schedule"),
        "Kit schedule should be present when existing workflow has none"
    );
}

#[test]
fn init_workflows_overwrite_preserves_wait_minutes() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    // Create an existing workflow with a custom wait_minutes default
    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    let existing_workflow = r#"name: Jules Workflows

on:
  workflow_dispatch:
    inputs:
      wait_minutes:
        default: 42
        description: Custom wait time

jobs:
  test: {}
"#;
    fs::write(&workflow_path, existing_workflow).unwrap();

    ctx.cli().args(["init", "workflows", "--remote", "--overwrite"]).assert().success();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    // The preserved default should be present
    assert!(
        updated_workflow.contains("default: 42"),
        "Custom wait_minutes default should be preserved"
    );
    // The kit content should still be present
    assert!(updated_workflow.contains("Jules"), "Workflow name should be present");
}
