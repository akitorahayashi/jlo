use crate::harness::TestContext;
use crate::harness::jlo_config;
use jlo::{WorkflowRunnerMode, init_workflows_at};
use std::fs;

#[test]
fn init_workflows_generates_timing_from_config() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    // Create an existing workflow with a custom schedule.
    let workflow_path = root.join(".github/workflows/jules-scheduled-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(
        &workflow_path,
        r#"name: Jules Scheduled Workflows

on:
  schedule:
    - cron: '0 12 * * 1-5'
    - cron: '0 6 * * 0'
  workflow_dispatch: {}

jobs:
  test: {}
"#,
    )
    .unwrap();

    let cron_entries = ["0 12 * * 1-5", "0 6 * * 0"];
    jlo_config::write_jlo_config(root, &cron_entries, 42);

    init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote()).unwrap();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    assert!(updated_workflow.contains("0 12 * * 1-5"));
    assert!(updated_workflow.contains("0 6 * * 0"));
    assert!(updated_workflow.contains("default: 42"));
    assert!(updated_workflow.contains("Jules"));
}

#[test]
fn init_workflows_requires_config() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    let result = init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote());
    assert!(result.is_err(), "Missing config should fail explicitly");
}

#[test]
fn init_workflows_fails_on_invalid_config() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    let jlo_dir = root.join(".jlo");
    fs::create_dir_all(&jlo_dir).unwrap();
    fs::write(jlo_dir.join("config.toml"), "invalid = [").unwrap();

    let result = init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote());
    assert!(result.is_err(), "Invalid config should fail explicitly");
}

#[test]
fn init_workflows_overwrites_invalid_existing_workflow() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    jlo_config::write_jlo_config(root, &[jlo_config::DEFAULT_TEST_CRON], 30);

    let workflow_path = root.join(".github/workflows/jules-scheduled-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(&workflow_path, "name: [invalid\n  yaml: content").unwrap();

    init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote()).unwrap();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    assert!(updated_workflow.contains("Jules Scheduled Workflows"));
}
