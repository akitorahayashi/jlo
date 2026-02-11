mod common;

use common::TestContext;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml::Value;

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

fn read_scheduled_roles(root: &Path, layer: &str) -> Vec<String> {
    let content = fs::read_to_string(root.join(".jlo/scheduled.toml")).unwrap();
    let value: Value = toml::from_str(&content).unwrap();

    let roles = value
        .get(layer)
        .and_then(|layer_value| layer_value.get("roles"))
        .and_then(|roles_value| roles_value.as_array())
        .cloned()
        .unwrap_or_default();

    roles
        .into_iter()
        .filter_map(|role_value| {
            role_value.get("name").and_then(|name| name.as_str()).map(|name| name.to_string())
        })
        .collect()
}

#[test]
fn test_cli_lifecycle() {
    let ctx = TestContext::new();

    // 1. Init
    ctx.cli()
        .args(["init", "--remote"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .jlo/"));

    // 2. Bootstrap
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Verify Structure (formerly init_creates_jules_directory & verify_scaffold_integrity)
    ctx.assert_jlo_exists();
    ctx.assert_jules_exists();
    assert!(ctx.read_version().is_some());
    ctx.assert_layer_structure_exists();
    ctx.assert_default_scheduled_roles_exist();
    ctx.assert_exchange_structure_exists();
    ctx.assert_events_structure_exists();
    ctx.assert_requirements_directory_exists();
    ctx.assert_contracts_exist();

    // Verify specific files
    let root_files = ["JULES.md", "README.md", ".jlo-version", "github-labels.json"];
    for file in root_files {
        assert!(ctx.jules_path().join(file).exists(), "{} should exist in .jules/ (Runtime)", file);
    }
    assert!(ctx.jlo_path().join("config.toml").exists());

    // 3. Doctor (formerly doctor_passes_on_fresh_workspace)
    ctx.cli().args(["doctor"]).assert().success();

    // 4. Create Role (formerly create_role_succeeds)
    ctx.cli()
        .args(["create", "observers", "custom-role"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created new"));

    let role_path = ctx.jlo_path().join("roles/observers/custom-role/role.yml");
    assert!(role_path.exists(), "Role should exist in .jlo/");
    let roles = read_scheduled_roles(ctx.work_dir(), "observers");
    assert!(roles.contains(&"custom-role".to_string()));

    // 5. Update (formerly update_succeeds_when_current)
    ctx.cli().args(["update"]).assert().success();

    // 6. Add Role (formerly add_role_installs_and_updates_schedule)
    ctx.cli()
        .args(["add", "observers", "pythonista"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added new"));

    let role_path = ctx.jlo_path().join("roles/observers/pythonista/role.yml");
    assert!(role_path.exists());
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
    // Use git command directly to setup user and commit
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

    ctx.cli()
        .args(["deinit"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed .jlo/ control plane"))
        .stdout(predicate::str::contains("Deleted local 'jules' branch"));

    let jlo_path = ctx.work_dir().join(".jlo");
    assert!(!jlo_path.exists(), ".jlo/ should be removed after deinit");
}

#[test]
fn create_role_fails_for_invalid_layer() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli()
        .args(["create", "invalid", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid layer"));
}

#[test]
fn create_role_fails_for_existing_role() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    // Create a role first
    ctx.cli().args(["create", "observers", "my-obs"]).assert().success();

    // Attempt duplicate creation
    ctx.cli()
        .args(["create", "observers", "my-obs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn create_role_fails_without_workspace() {
    let ctx = TestContext::new();

    ctx.cli()
        .args(["create", "observers", "test"])
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
        predicate::str::contains("[aliases: i]").and(predicate::str::contains("[aliases: cr]")),
    );
}

#[test]
fn doctor_reports_schema_errors() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    let event_dir = ctx.work_dir().join(".jules/exchange/events/pending");
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

#[test]
fn workflow_generate_writes_expected_files() {
    let ctx = TestContext::new();

    write_jlo_config(ctx.work_dir(), &[DEFAULT_CRON], 30);

    let output_dir = ctx.work_dir().join(".tmp/workflow-scaffold-generate/remote");
    ctx.cli()
        .args(["workflow", "generate", "remote", "--output-dir"])
        .arg(&output_dir)
        .assert()
        .success();

    assert!(
        output_dir.join(".github/workflows/jules-workflows.yml").exists(),
        "Generated workflow file should exist"
    );
}

#[test]
fn workflow_generate_uses_default_output_dir() {
    let ctx = TestContext::new();

    write_jlo_config(ctx.work_dir(), &[DEFAULT_CRON], 30);

    ctx.cli().args(["workflow", "generate", "remote"]).assert().success();

    // Default output writes directly to repository .github/
    let default_path = ctx.work_dir().join(".github/workflows/jules-workflows.yml");
    assert!(default_path.exists(), "Default generate output should exist in .github/");
}

#[test]
fn workflow_generate_overwrites_by_default() {
    let ctx = TestContext::new();

    write_jlo_config(ctx.work_dir(), &[DEFAULT_CRON], 30);

    let output_dir = ctx.work_dir().join(".tmp/workflow-scaffold-generate/overwrite");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(output_dir.join("old.txt"), "old content").unwrap();

    // Generate overwrites by default (no --overwrite flag needed)
    ctx.cli()
        .args(["workflow", "generate", "remote", "--output-dir"])
        .arg(&output_dir)
        .assert()
        .success();

    assert!(
        output_dir.join(".github/workflows/jules-workflows.yml").exists(),
        "Generated workflow file should exist after overwrite"
    );
}

// =============================================================================
// Setup Command Tests
// =============================================================================

#[test]
fn init_creates_setup_structure() {
    // This is partially redundant with test_cli_lifecycle but checks specific files in .jlo/setup
    // Can merge it but keeping it separate for clarity on setup structure is also fine.
    // Actually let's just merge checking into lifecycle if it's not already checked.
    // I added partial checks in lifecycle but not all. I'll keep this one as it's fast.
    let ctx = TestContext::new();

    ctx.cli()
        .args(["init", "--remote"])
        .assert()
        .success();

    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Verify setup config in .jlo
    assert!(ctx.work_dir().join(".jlo/setup").exists());
    assert!(ctx.work_dir().join(".jlo/setup/tools.yml").exists());
    assert!(ctx.work_dir().join(".jlo/setup/.gitignore").exists());
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

    // Write tools config in .jlo
    let tools_yml = ctx.work_dir().join(".jlo/setup/tools.yml");
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
        .stdout(predicate::str::contains("just"));
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
// Run Implementer Tests
// =============================================================================

#[test]
fn run_implementer_requires_requirement_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.cli()
        .args(["run", "implementer"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn run_planner_requires_requirement_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.cli()
        .args(["run", "planner"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn run_implementer_with_missing_requirement_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    ctx.cli()
        .args([
            "run",
            "implementer",
            "--requirement",
            ".jules/exchange/requirements/nonexistent.yml",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Requirement file not found"));
}

#[test]
fn run_implementer_prompt_preview_with_requirement_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Create a test requirement file in flat exchange
    let requirement_dir = ctx.work_dir().join(".jules/exchange/requirements");
    std::fs::create_dir_all(&requirement_dir).unwrap();
    let requirement_path = requirement_dir.join("test_requirement.yml");
    std::fs::write(
        &requirement_path,
        "fingerprint: test_requirement\nid: test_requirement\ntitle: Test Requirement\nlabel: bugs\nstatus: open\n",
    )
    .unwrap();

    ctx.cli()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "run",
            "implementer",
            "--requirement",
            ".jules/exchange/requirements/test_requirement.yml",
            "--prompt-preview",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Prompt Preview: Implementer"))
        .stdout(predicate::str::contains("Would execute 1 session"));
}

#[test]
fn run_planner_prompt_preview_with_requirement_file() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Create a test requirement file in flat exchange
    let requirement_dir = ctx.work_dir().join(".jules/exchange/requirements");
    std::fs::create_dir_all(&requirement_dir).unwrap();
    let requirement_path = requirement_dir.join("test_requirement.yml");
    std::fs::write(
        &requirement_path,
        "fingerprint: test_requirement\nid: test_requirement\ntitle: Test Requirement\nstatus: open\nrequires_deep_analysis: true\n",
    )
    .unwrap();

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
        .stdout(predicate::str::contains("Target Range"));
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
fn update_cli_conflicts_with_prompt_preview() {
    let ctx = TestContext::new();
    ctx.cli()
        .args(["update", "--cli", "--prompt-preview"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
