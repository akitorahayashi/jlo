mod common;

use common::TestContext;
use jlo::{WorkflowRunnerMode, init_workflows_at};
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use yamllint_rs::{FileProcessor, ProcessingOptions, Severity};

const DEFAULT_CRON: &str = "0 20 * * *";

fn write_jlo_config(root: &Path, crons: &[&str], wait_minutes_default: u32) {
    let jlo_dir = root.join(".jlo");
    fs::create_dir_all(&jlo_dir).unwrap();

    let cron_entries =
        crons.iter().map(|cron| format!("\"{}\"", cron)).collect::<Vec<_>>().join(", ");

    let content = format!(
        r#"[run]
default_branch = "main"
jules_branch = "jules"

[workflow]
cron = [{}]
wait_minutes_default = {}
"#,
        cron_entries, wait_minutes_default
    );

    fs::write(jlo_dir.join("config.toml"), content).unwrap();
}

fn ensure_jlo_config(root: &Path) {
    let config_path = root.join(".jlo/config.toml");
    if !config_path.exists() {
        write_jlo_config(root, &[DEFAULT_CRON], 30);
    }
}

#[test]
fn init_workflows_installs_remote_scaffold() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    let root = ctx.work_dir();
    assert!(root.join(".github/workflows/jules-workflows.yml").exists());
    assert!(root.join(".github/actions/install-jlo/action.yml").exists());
    assert!(
        !root.join(".github/actions/wait/action.yml").exists(),
        "Workflow scaffold should not include legacy wait action"
    );
    assert!(
        !root.join(".github/workflows/jules-workflows/components").exists(),
        "Workflow scaffold should not include workflow template components"
    );
    // Scripts directory no longer ships with workflow scaffold
    assert!(
        !root.join(".github/scripts").exists(),
        "Workflow scaffold should not include .github/scripts"
    );

    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(workflow.contains("Run observers"), "Should run observers");
    assert!(workflow.contains("Run decider"), "Should run decider");
    assert!(workflow.contains("Run planner"), "Should run planner");
    assert!(workflow.contains("Run implementer"), "Should run implementer");
    assert!(workflow.contains("Run innovators (first pass)"), "Should run innovators first pass");
    assert!(workflow.contains("Run innovators (second pass)"), "Should run innovators second pass");
    assert!(workflow.contains("Publish proposals"), "Should publish proposals");
    // Verify no script references
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
fn init_workflows_installs_self_hosted_scaffold() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--self-hosted"]).assert().success();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();

    assert!(workflow.contains("runs-on: self-hosted"), "Should use self-hosted runner");
    assert!(!workflow.contains("strategy: matrix"), "Should not use matrix strategy");
    assert!(workflow.contains("Run observers"), "Should run observers");
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

    write_jlo_config(root, &[DEFAULT_CRON], 30);

    let scaffold_workflow = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(scaffold_workflow.parent().unwrap()).unwrap();
    fs::write(&scaffold_workflow, "old workflow").unwrap();

    let unrelated_workflow = root.join(".github/workflows/unrelated.yml");
    fs::write(&unrelated_workflow, "keep me").unwrap();

    let scaffold_action = root.join(".github/actions/install-jlo/action.yml");
    fs::create_dir_all(scaffold_action.parent().unwrap()).unwrap();
    fs::write(&scaffold_action, "old action").unwrap();

    let unrelated_action = root.join(".github/actions/custom/action.yml");
    fs::create_dir_all(unrelated_action.parent().unwrap()).unwrap();
    fs::write(&unrelated_action, "custom action").unwrap();

    // Use API directly — testing workflow scaffold re-install over existing files
    init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote()).unwrap();

    let updated_workflow = fs::read_to_string(&scaffold_workflow).unwrap();
    assert!(updated_workflow.contains("Jules Workflows"));

    let updated_action = fs::read_to_string(&scaffold_action).unwrap();
    assert!(updated_action.contains("Install jlo"));

    let unrelated_content = fs::read_to_string(&unrelated_workflow).unwrap();
    assert_eq!(unrelated_content, "keep me");

    let unrelated_action_content = fs::read_to_string(&unrelated_action).unwrap();
    assert_eq!(unrelated_action_content, "custom action");
}

#[test]
fn init_workflows_generates_timing_from_config() {
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

    let cron_entries = ["0 12 * * 1-5", "0 6 * * 0"];
    write_jlo_config(root, &cron_entries, 42);

    init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote()).unwrap();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    assert!(updated_workflow.contains("0 12 * * 1-5"), "Schedule should use config values");
    assert!(updated_workflow.contains("0 6 * * 0"), "Schedule should use config values");
    assert!(
        updated_workflow.contains("default: 42"),
        "wait_minutes default should use config values"
    );
    assert!(updated_workflow.contains("Jules"), "Workflow name should be present");
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

    write_jlo_config(root, &[DEFAULT_CRON], 30);

    let workflow_path = root.join(".github/workflows/jules-workflows.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(&workflow_path, "name: [invalid\n  yaml: content").unwrap();

    init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote()).unwrap();

    let updated_workflow = fs::read_to_string(&workflow_path).unwrap();
    assert!(updated_workflow.contains("Jules Workflows"));
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

    assert!(workflow.contains("vars.JLO_PAUSED"), "Workflow should use JLO_PAUSED variable");
    assert!(
        !workflow.contains("vars.JULES_PAUSED"),
        "Workflow should not use legacy JULES_PAUSED variable"
    );

    // Pause gating applies only to schedule events
    assert!(
        workflow
            .contains("(vars.JLO_PAUSED || 'false') != 'true' || github.event_name != 'schedule'"),
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

    // Verify all workflow files in the scaffold
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
    for action_dir in ["install-jlo", "configure-git"] {
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

#[test]
fn init_workflows_enforces_explicit_branch_contract() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    let root = ctx.work_dir();

    let primary = fs::read_to_string(root.join(".github/workflows/jules-workflows.yml")).unwrap();
    assert!(
        primary.contains("JLO_TARGET_BRANCH"),
        "Primary workflow should reference JLO_TARGET_BRANCH"
    );
    assert!(
        primary.contains("JULES_WORKER_BRANCH"),
        "Primary workflow should reference JULES_WORKER_BRANCH"
    );

    let sync = fs::read_to_string(root.join(".github/workflows/jules-sync.yml")).unwrap();
    assert!(sync.contains("JLO_TARGET_BRANCH"), "Sync workflow should reference JLO_TARGET_BRANCH");
    assert!(
        sync.contains("JULES_WORKER_BRANCH"),
        "Sync workflow should reference JULES_WORKER_BRANCH"
    );

    for entry in fs::read_dir(root.join(".github/workflows")).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|ext| ext == "yml") {
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains("github.event.repository.default_branch"),
                "Workflow {} should not reference github.event.repository.default_branch",
                entry.path().display()
            );
            assert!(
                !content.contains(".jlo-control"),
                "Workflow {} should not reference .jlo-control",
                entry.path().display()
            );
        }
    }

    for action_dir in ["install-jlo", "configure-git"] {
        let action_path = root.join(format!(".github/actions/{}/action.yml", action_dir));
        if action_path.exists() {
            let content = fs::read_to_string(&action_path).unwrap();
            assert!(
                !content.contains("github.event.repository.default_branch"),
                "Action {} should not reference github.event.repository.default_branch",
                action_dir
            );
            assert!(
                !content.contains(".jlo-control"),
                "Action {} should not reference .jlo-control",
                action_dir
            );
        }
    }
}

#[test]
fn workflow_templates_parse_with_serde_yaml() {
    for mode in ["remote", "self-hosted"] {
        let ctx = TestContext::new();
        let output_dir = generate_workflow_scaffold(&ctx, mode, "parse");

        let files = collect_yaml_files(&output_dir);
        assert!(
            !files.is_empty(),
            "Generated workflow scaffold produced no YAML files for {} mode",
            mode
        );

        for file in files {
            let content = fs::read_to_string(&file)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", file.display(), e));
            let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(&content);
            assert!(
                result.is_ok(),
                "{} ({} mode) failed to parse with serde_yaml: {}",
                file.display(),
                mode,
                result.unwrap_err()
            );
        }
    }
}

#[test]
fn workflow_templates_pass_yaml_lint_remote() {
    validate_yaml_lint("remote");
}

#[test]
fn workflow_templates_pass_yaml_lint_self_hosted() {
    validate_yaml_lint("self-hosted");
}

fn validate_yaml_lint(mode: &str) {
    let ctx = TestContext::new();
    let output_dir = generate_workflow_scaffold(&ctx, mode, "lint");

    let files = collect_yaml_files(&output_dir);
    assert!(
        !files.is_empty(),
        "Generated workflow scaffold produced no YAML files for {} mode",
        mode
    );

    let mut config = yamllint_rs::config::Config::new();
    config.set_rule_enabled("line-length", false);
    config.set_rule_enabled("indentation", false);
    config.set_rule_enabled("truthy", false);
    config.set_rule_enabled("document-start", false);
    config.set_rule_enabled("comments", false);

    let processor = FileProcessor::with_config(ProcessingOptions::default(), config);

    let mut errors = Vec::new();

    for file in files {
        match processor.process_file(&file) {
            Ok(result) => {
                let issues: Vec<_> = result
                    .issues
                    .iter()
                    .filter(|(issue, _)| issue.severity == Severity::Error)
                    .collect();

                if !issues.is_empty() {
                    let mut msg = format!("\n  {}:", file.display());
                    for (issue, line) in &issues {
                        msg.push_str(&format!(
                            "\n    L{}: {} - {}",
                            issue.line, issue.message, line
                        ));
                    }
                    errors.push(msg);
                }
            }
            Err(e) => {
                errors.push(format!("\n  {}: failed to lint - {}", file.display(), e));
            }
        }
    }

    assert!(errors.is_empty(), "YAML lint errors for {} mode:{}", mode, errors.join(""));
}

fn generate_workflow_scaffold(ctx: &TestContext, mode: &str, suffix: &str) -> PathBuf {
    let output_dir = ctx
        .work_dir()
        .join(".tmp/workflow-scaffold-generate/tests")
        .join(format!("{}-{}", mode, suffix));

    ensure_jlo_config(ctx.work_dir());

    let mut command = ctx.cli();
    command
        .args(["workflow", "generate", mode, "--output-dir"])
        .arg(&output_dir)
        .assert()
        .success();

    output_dir
}

fn collect_yaml_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_yaml_files_into(root, &mut files);
    files
}

fn collect_yaml_files_into(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root)
        .unwrap_or_else(|e| panic!("Failed to read directory {}: {}", root.display(), e));

    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            panic!("Failed to read directory entry in {}: {}", root.display(), e)
        });
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files_into(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "yml" || ext == "yaml")
        {
            files.push(path);
        }
    }
}

#[test]
fn automerge_delegates_to_jlo_command() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    let root = ctx.work_dir();
    let automerge = fs::read_to_string(root.join(".github/workflows/jules-automerge.yml")).unwrap();

    // Policy logic is delegated to jlo, not inline bash
    assert!(
        automerge.contains("jlo workflow gh pr enable-automerge"),
        "Automerge workflow must delegate to `jlo workflow gh pr enable-automerge`"
    );
    assert!(
        automerge.contains("concurrency:"),
        "Automerge workflow must serialize auto-merge jobs with concurrency control"
    );
    assert!(
        automerge.contains("jules-automerge-"),
        "Automerge concurrency group should scope by PR base branch"
    );
    assert!(
        automerge.contains("cancel-in-progress: false"),
        "Automerge concurrency must queue runs instead of canceling in-progress jobs"
    );

    // Must NOT contain inline bash policy logic
    assert!(
        !automerge.contains("allowed_prefixes"),
        "Automerge workflow must not contain inline prefix matching (delegated to jlo)"
    );
    assert!(
        !automerge.contains("git ls-tree"),
        "Automerge workflow must not use dynamic contract scanning (git ls-tree)"
    );
    assert!(
        !automerge.contains("in_scope"),
        "Automerge workflow must not contain inline scope checking (delegated to jlo)"
    );
}
