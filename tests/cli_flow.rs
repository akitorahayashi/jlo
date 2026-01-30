mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn user_can_init_and_create_custom_role() {
    let ctx = TestContext::new();

    // Initialize workspace
    ctx.cli().arg("init").assert().success();

    // All built-in roles should exist after init in their layers
    ctx.assert_all_builtin_roles_exist();

    // Create a custom observer role
    ctx.cli()
        .args(["template", "-l", "observers", "-n", "security"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created new role"));

    ctx.assert_role_in_layer_exists("observers", "security");
}

#[test]
#[serial]
fn user_can_use_command_aliases() {
    let ctx = TestContext::new();

    // Use 'i' alias for init
    ctx.cli().arg("i").assert().success();

    // Use 'tp' alias for template (with a multi-role layer)
    ctx.cli().args(["tp", "-l", "deciders", "-n", "my-decider"]).assert().success();

    ctx.assert_role_in_layer_exists("deciders", "my-decider");
}

#[test]
#[serial]
fn init_creates_complete_layer_structure() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    // Verify layer structure
    ctx.assert_jules_exists();
    ctx.assert_layer_structure_exists();
    ctx.assert_events_structure_exists();
    ctx.assert_issues_directory_exists();
    ctx.assert_all_builtin_roles_exist();

    // Verify observers have notes and feedbacks directories
    let jules = ctx.jules_path();
    assert!(jules.join("roles/observers/taxonomy/notes").exists());
    assert!(jules.join("roles/observers/taxonomy/feedbacks").exists());
    assert!(jules.join("roles/observers/data_arch/notes").exists());
    assert!(jules.join("roles/observers/data_arch/feedbacks").exists());
    assert!(jules.join("roles/observers/qa/notes").exists());
    assert!(jules.join("roles/observers/qa/feedbacks").exists());

    // Verify non-observers don't have notes, feedbacks, or role.yml
    assert!(!jules.join("roles/deciders/triage_generic/notes").exists());
    assert!(!jules.join("roles/deciders/triage_generic/feedbacks").exists());
    assert!(!jules.join("roles/deciders/triage_generic/role.yml").exists());

    // Single-role layers have flat structure (no role subdirectory)
    assert!(jules.join("roles/planners/prompt.yml").exists());
    assert!(jules.join("roles/planners/contracts.yml").exists());
    assert!(jules.join("roles/implementers/prompt.yml").exists());
    assert!(jules.join("roles/implementers/contracts.yml").exists());
}

#[test]
#[serial]
fn template_creates_observer_with_notes() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli().args(["template", "-l", "observers", "-n", "custom-obs"]).assert().success();

    // Observer roles should have notes and feedbacks directories, plus role.yml
    let role_path = ctx.jules_path().join("roles/observers/custom-obs");
    let notes_path = role_path.join("notes");
    let feedbacks_path = role_path.join("feedbacks");
    let role_yml = role_path.join("role.yml");
    assert!(notes_path.exists(), "Observer role should have notes directory");
    assert!(feedbacks_path.exists(), "Observer role should have feedbacks directory");
    assert!(role_yml.exists(), "Observer role should have role.yml");
}

#[test]
#[serial]
fn template_rejects_single_role_layers() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    // Planners are single-role and should not accept template creation
    ctx.cli()
        .args(["template", "-l", "planners", "-n", "custom-planner"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-role"));

    // Implementers are single-role and should not accept template creation
    ctx.cli()
        .args(["template", "-l", "implementers", "-n", "custom-impl"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-role"));
}
