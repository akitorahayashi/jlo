mod common;

use common::TestContext;
use predicates::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn init_creates_jules_directory() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["init", "--remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .jlo/"));

    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.assert_jlo_exists();
    ctx.assert_jules_exists();
    assert!(ctx.read_version().is_some());
    ctx.assert_layer_structure_exists();
    ctx.assert_all_builtin_roles_exist();
    ctx.assert_exchange_structure_exists();
    ctx.assert_events_structure_exists();
    ctx.assert_issues_directory_exists();
    ctx.assert_contracts_exist();
}

#[test]
fn init_fails_if_jules_exists() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli()
        .args(["init", "--remote"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn deinit_fails_on_jules_branch() {
    let ctx = TestContext::new();

    // Must be on 'jules' branch for deinit to reject
    ctx.git_checkout_branch("jules", true);

    ctx.cli()
        .args(["deinit"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cannot deinit while on branch"));
}

#[test]
fn deinit_removes_workflows_and_branch() {
    let ctx = TestContext::new();
    let seed_file = ctx.work_dir().join("seed.txt");
    fs::write(&seed_file, "seed").unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git config email failed");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git config name failed");
    Command::new("git")
        .args(["add", "seed.txt"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git add failed");
    Command::new("git")
        .args(["commit", "-m", "seed"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git commit failed");

    // Init on the control branch (already on main/master after git init)
    ctx.cli().args(["init", "--remote"]).assert().success();

    // Create a 'jules' branch so deinit can delete it, then return to control branch
    ctx.git_checkout_branch("jules", true);
    let switch_back = Command::new("git")
        .args(["checkout", "-"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git checkout - failed");
    assert!(switch_back.status.success(), "switch back to control branch failed");

    let workflow_path = ctx.work_dir().join(".github/workflows/jules-workflows.yml");
    let action_path = ctx.work_dir().join(".github/actions/install-jlo/action.yml");
    let jlo_path = ctx.work_dir().join(".jlo");
    assert!(workflow_path.exists(), "workflow kit file should exist before deinit");
    assert!(action_path.exists(), "workflow action should exist before deinit");
    assert!(jlo_path.exists(), ".jlo/ should exist before deinit");

    ctx.cli()
        .args(["deinit"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed .jlo/ control plane"))
        .stdout(predicate::str::contains("Deleted local 'jules' branch"));

    assert!(!workflow_path.exists(), "workflow kit file should be removed");
    assert!(!action_path.exists(), "workflow action should be removed");
    assert!(!jlo_path.exists(), ".jlo/ should be removed after deinit");

    let output = Command::new("git")
        .args(["branch", "--list", "jules"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git branch list failed");
    assert!(
        String::from_utf8_lossy(&output.stdout).trim().is_empty(),
        "jules branch should be deleted"
    );
}

#[test]
fn create_role_succeeds() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli()
        .args(["create", "role", "observers", "custom-role"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created new role"));

    let role_path = ctx.jlo_path().join("roles/observers/roles/custom-role/role.yml");
    assert!(role_path.exists(), "Role should exist in .jlo/");
}

#[test]
fn create_workstream_succeeds() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli()
        .args(["create", "workstream", "my-stream"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created new workstream"));

    let ws_path = ctx.jlo_path().join("workstreams/my-stream/scheduled.toml");
    assert!(ws_path.exists(), "Workstream should exist in .jlo/");
}

#[test]
fn create_role_fails_for_invalid_layer() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli()
        .args(["create", "role", "invalid", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid layer"));
}

#[test]
fn create_role_fails_for_existing_role() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    // Create a role first
    ctx.cli().args(["create", "role", "observers", "my-obs"]).assert().success();

    // Attempt duplicate creation
    ctx.cli()
        .args(["create", "role", "observers", "my-obs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn create_role_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["create", "role", "observers", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace"));
}

#[test]
fn create_workstream_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["create", "workstream", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace"));
}

#[test]
fn version_flag_works() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_visible_aliases() {
    let ctx = TestContext::new();

    ctx.cli().arg("--help").assert().success().stdout(
        predicate::str::contains("[aliases: i]").and(predicate::str::contains("[aliases: c]")),
    );
}

#[test]
fn doctor_passes_on_fresh_workspace() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn doctor_reports_schema_errors() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    let event_dir = ctx.work_dir().join(".jules/workstreams/generic/exchange/events/pending");
    std::fs::create_dir_all(&event_dir).unwrap();
    let event_path = event_dir.join("bad-event.yml");
    std::fs::write(
        &event_path,
        "schema_version: 1\nid: abc123\nissue_id: \"\"\ncreated_at: 2026-01-01\nauthor_role: tester\nconfidence: low\ntitle: Bad event\nstatement: too short\nevidence: []\n",
    )
    .unwrap();

    ctx.cli()
        .args(["doctor"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("evidence must have entries"));
}

// =============================================================================
// Setup Command Tests
// =============================================================================

#[test]
fn init_creates_setup_structure() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["init", "--remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .jlo/"));

    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    assert!(ctx.work_dir().join(".jules/setup").exists());
    assert!(ctx.work_dir().join(".jules/setup/tools.yml").exists());
    assert!(ctx.work_dir().join(".jules/setup/.gitignore").exists());
}

#[test]
fn setup_gen_requires_init() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "gen"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Setup not initialized"));
}

#[test]
fn setup_gen_produces_script() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Write tools config
    let tools_yml = ctx.work_dir().join(".jules/setup/tools.yml");
    std::fs::write(&tools_yml, "tools:\n  - just\n").unwrap();

    ctx.cli()
        .args(["setup", "gen"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated install.sh"));

    assert!(ctx.work_dir().join(".jules/setup/install.sh").exists());
    assert!(ctx.work_dir().join(".jules/setup/env.toml").exists());
}

#[test]
fn setup_list_shows_components() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available components:"))
        .stdout(predicate::str::contains("just"))
        .stdout(predicate::str::contains("swift"))
        .stdout(predicate::str::contains("uv"));
}

#[test]
fn setup_list_detail_shows_info() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list", "--detail", "just"])
        .assert()
        .success()
        .stdout(predicate::str::contains("just:"))
        .stdout(predicate::str::contains("Install Script:"));
}

#[test]
fn setup_list_detail_not_found() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["setup", "list", "--detail", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// Run Implementers Tests
// =============================================================================

#[test]
fn run_implementers_requires_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.cli()
        .args(["run", "implementers"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn run_planners_requires_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.cli()
        .args(["run", "planners"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn run_implementers_with_missing_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.cli()
        .args(["run", "implementers", ".jules/workstreams/generic/issues/nonexistent.yml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue file not found"));
}

#[test]
fn run_implementers_prompt_preview_with_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Create a test issue file in workstreams
    let issue_dir = ctx.work_dir().join(".jules/workstreams/generic/issues/medium");
    std::fs::create_dir_all(&issue_dir).unwrap();
    let issue_path = issue_dir.join("test_issue.yml");
    std::fs::write(
        &issue_path,
        "fingerprint: test_issue\nid: test_issue\ntitle: Test Issue\nstatus: open\n",
    )
    .unwrap();

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "run",
            "implementers",
            ".jules/workstreams/generic/issues/medium/test_issue.yml",
            "--prompt-preview",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Local Dispatch"))
        .stdout(predicate::str::contains("Would dispatch workflow"));
}

#[test]
fn run_planners_prompt_preview_with_issue_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Create a test issue file in workstreams
    let issue_dir = ctx.work_dir().join(".jules/workstreams/generic/issues/medium");
    std::fs::create_dir_all(&issue_dir).unwrap();
    let issue_path = issue_dir.join("test_issue.yml");
    std::fs::write(
        &issue_path,
        "fingerprint: test_issue\nid: test_issue\ntitle: Test Issue\nstatus: open\nrequires_deep_analysis: true\n",
    )
    .unwrap();

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "run",
            "planners",
            ".jules/workstreams/generic/issues/medium/test_issue.yml",
            "--prompt-preview",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Local Dispatch"))
        .stdout(predicate::str::contains("Would dispatch workflow"));
}

#[test]
fn run_narrator_prompt_preview() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Configure git user for commits
    let output = std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git config email failed");
    assert!(output.status.success(), "git config user.email failed");

    let output = std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git config name failed");
    assert!(output.status.success(), "git config user.name failed");

    // Create first commit (includes both .jules/ and README.md)
    std::fs::write(ctx.work_dir().join("README.md"), "# Test Project\n").unwrap();
    let output = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git add failed");
    assert!(output.status.success(), "git add failed");

    let output = std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git commit failed");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Create second commit with codebase changes to have a non-empty range
    std::fs::write(ctx.work_dir().join("README.md"), "# Test Project\n\nUpdated content.\n")
        .unwrap();
    let output = std::process::Command::new("git")
        .args(["add", "README.md"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git add failed");
    assert!(output.status.success(), "git add failed");

    let output = std::process::Command::new("git")
        .args(["commit", "-m", "update readme"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git commit failed");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args(["run", "narrator", "--prompt-preview"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Narrator"))
        .stdout(predicate::str::contains("Git Context"));
}

#[test]
fn run_narrator_skips_when_no_codebase_changes() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Configure git user for commits
    let output = std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git config email failed");
    assert!(output.status.success(), "git config user.email failed");

    let output = std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git config name failed");
    assert!(output.status.success(), "git config user.name failed");

    // Create an initial commit with ONLY .jules/ changes (no codebase changes)
    let output = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git add failed");
    assert!(output.status.success(), "git add failed");

    let output = std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(ctx.work_dir())
        .output()
        .expect("git commit failed");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args(["run", "narrator", "--prompt-preview"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No codebase changes detected"));
}

// =============================================================================
// Update Command Tests
// =============================================================================

#[test]
fn update_requires_workspace() {
    let ctx = TestContext::new();

    ctx.cli().args(["update"]).assert().failure().stderr(predicate::str::contains("No .jlo/"));
}

#[test]
fn update_prompt_preview_shows_plan() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    // Simulate an older version to trigger update logic
    let version_file = ctx.work_dir().join(".jlo").join(".jlo-version");
    std::fs::write(&version_file, "0.0.0").expect("write version");

    ctx.cli()
        .args(["update", "--prompt-preview"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview"));
}

#[test]
fn update_noop_when_current() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    // When already at current version, update may still refresh workflow kit or be completely done
    ctx.cli().args(["update"]).assert().success();
}

#[test]
fn update_alias_works() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli().args(["u", "--prompt-preview"]).assert().success();
}
