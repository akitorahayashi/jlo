mod common;

use common::TestContext;
use jlo::{WorkflowRunnerMode, init_workflows_at};
use predicates::prelude::*;
use std::fs;

#[test]
fn init_workflows_installs_remote_kit() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    let root = ctx.work_dir();
    assert!(root.join(".github/workflows/jules-workflows.yml").exists());
    assert!(root.join(".github/actions/install-jlo/action.yml").exists());
    assert!(
        !root.join(".github/actions/wait/action.yml").exists(),
        "Workflow kit should not include legacy wait action"
    );
    assert!(
        !root.join(".github/workflows/jules-workflows/components").exists(),
        "Workflow kit should not include workflow template components"
    );
    // Scripts directory no longer ships with workflow kit
    assert!(
        !root.join(".github/scripts").exists(),
        "Workflow kit should not include .github/scripts"
    );

    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(
        workflow.contains("Run observers for each workstream"),
        "Should run observers for each workstream"
    );
    assert!(
        workflow.contains("Run deciders for each pending workstream"),
        "Should run deciders for each pending workstream"
    );
    assert!(
        workflow.contains("Run planners for each workstream"),
        "Should run planners for each workstream"
    );
    assert!(
        workflow.contains("Run implementers for each workstream"),
        "Should run implementers for each workstream"
    );
    assert!(
        workflow.contains("Run innovators (first pass) for each workstream"),
        "Should run innovators first pass for each workstream"
    );
    assert!(
        workflow.contains("Run innovators (second pass) for each workstream"),
        "Should run innovators second pass for each workstream"
    );
    assert!(
        workflow.contains("Publish proposals for each workstream"),
        "Should publish proposals for each workstream"
    );
    // Verify no script references
    assert!(
        !workflow.contains(".github/scripts/"),
        "Workflow should not reference .github/scripts/"
    );
    assert!(
        !workflow.contains("{% include"),
        "Rendered workflow should not contain template include directives"
    );
}

#[test]
fn init_workflows_installs_self_hosted_kit() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--self-hosted"]).assert().success();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();

    assert!(workflow.contains("runs-on: self-hosted"), "Should use self-hosted runner");
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(
        workflow.contains("Run observers for each workstream"),
        "Should run observers for each workstream"
    );
}

#[test]
fn init_requires_runner_mode() {
    let ctx = TestContext::new();

    ctx.cli().args(["init"]).assert().failure().stderr(predicate::str::contains("required"));
}

#[test]
fn init_workflows_respects_unrelated_files() {
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

    // Use API directly — testing workflow kit re-install over existing files
    init_workflows_at(root.to_path_buf(), WorkflowRunnerMode::Remote).unwrap();

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
fn init_workflows_preserves_schedule() {
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

    init_workflows_at(root.to_path_buf(), WorkflowRunnerMode::Remote).unwrap();

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
fn init_workflows_fails_on_invalid_schedule() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    // Create an existing workflow with invalid YAML
    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    let invalid_yaml = "name: [invalid\n  yaml: content";
    fs::write(&workflow_path, invalid_yaml).unwrap();

    let result = init_workflows_at(root.to_path_buf(), WorkflowRunnerMode::Remote);
    assert!(result.is_err(), "Should fail on invalid YAML");
}

#[test]
fn init_workflows_keeps_existing_actions_when_schedule_parse_fails() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(&workflow_path, "name: [invalid\n  yaml: content").unwrap();

    let action_path = root.join(".github/actions/install-jlo/action.yml");
    fs::create_dir_all(action_path.parent().unwrap()).unwrap();
    fs::write(&action_path, "keep this action").unwrap();

    let _ = init_workflows_at(root.to_path_buf(), WorkflowRunnerMode::Remote);

    let action_content = fs::read_to_string(&action_path).unwrap();
    assert_eq!(action_content, "keep this action");
}

#[test]
fn init_workflows_uses_kit_schedule_when_none_exists() {
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

    init_workflows_at(root.to_path_buf(), WorkflowRunnerMode::Remote).unwrap();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    // The kit's default schedule should be present
    assert!(
        updated_workflow.contains("schedule"),
        "Kit schedule should be present when existing workflow has none"
    );
}

#[test]
fn init_workflows_preserves_wait_minutes() {
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

    init_workflows_at(root.to_path_buf(), WorkflowRunnerMode::Remote).unwrap();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    // The preserved default should be present
    assert!(
        updated_workflow.contains("default: 42"),
        "Custom wait_minutes default should be preserved"
    );
    // The kit content should still be present
    assert!(updated_workflow.contains("Jules"), "Workflow name should be present");
}

#[test]
fn init_workflows_includes_mock_support() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();

    // Mock inputs should be present
    assert!(workflow.contains("mock:"), "Should have mock input");
    assert!(workflow.contains("workflow_call:"), "Should support workflow_call trigger");

    // Mock environment variables should be set (JULES_MOCK_TAG auto-generated from run_id)
    assert!(workflow.contains("MOCK_MODE:"), "Should set MOCK_MODE env var");
    assert!(workflow.contains("JULES_MOCK_TAG:"), "Should set JULES_MOCK_TAG env var");
    assert!(workflow.contains("JLO_RUN_FLAGS:"), "Should set JLO_RUN_FLAGS env var");

    // Two-pass innovator cycle should be present for mock determinism
    assert!(workflow.contains("run-innovators-1:"), "Should have first innovator pass job");
    assert!(workflow.contains("run-innovators-2:"), "Should have second innovator pass job");
    assert!(workflow.contains("publish-proposals:"), "Should have publish-proposals job");

    // Entry point should include innovators
    assert!(workflow.contains("- innovators"), "Entry point choices should include innovators");
}

#[test]
fn init_workflows_uses_jlo_paused_not_jules_paused() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();

    assert!(
        workflow.contains("vars.JLO_PAUSED"),
        "Workflow should use JLO_PAUSED variable"
    );
    assert!(
        !workflow.contains("vars.JULES_PAUSED"),
        "Workflow should not use legacy JULES_PAUSED variable"
    );

    // Pause gating applies only to schedule events
    assert!(
        workflow.contains("vars.JLO_PAUSED != 'true' || github.event_name != 'schedule'"),
        "Pause gating should allow non-schedule events to proceed"
    );
}

#[test]
fn init_workflows_no_scripts_references() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    let root = ctx.work_dir();

    // Verify workflow templates do not reference .github/scripts
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();
    assert!(
        !workflow.contains(".github/scripts/"),
        "jules-workflows.yml should not reference .github/scripts/"
    );

    // Verify all workflow files in the kit
    for entry in fs::read_dir(root.join(".github/workflows")).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|ext| ext == "yml") {
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains(".github/scripts/"),
                "Workflow {} should not reference .github/scripts/",
                entry.path().display()
            );
        }
    }

    // Verify composite actions do not reference .github/scripts
    for action_dir in ["install-jlo", "configure-git", "run-implementer"] {
        let action_path = root.join(format!(".github/actions/{}/action.yml", action_dir));
        if action_path.exists() {
            let content = fs::read_to_string(&action_path).unwrap();
            assert!(
                !content.contains(".github/scripts/"),
                "Action {} should not reference .github/scripts/",
                action_dir
            );
        }
    }
}
