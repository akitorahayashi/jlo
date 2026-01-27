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

    // All 6 built-in roles should exist after init in their layers
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

    // Use 'tp' alias for template
    ctx.cli().args(["tp", "-l", "planners", "-n", "my-planner"]).assert().success();

    ctx.assert_role_in_layer_exists("planners", "my-planner");
}

#[test]
#[serial]
fn init_creates_complete_4_layer_structure() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    // Verify 4-layer structure
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
    assert!(!jules.join("roles/deciders/triage/notes").exists());
    assert!(!jules.join("roles/deciders/triage/feedbacks").exists());
    assert!(!jules.join("roles/deciders/triage/role.yml").exists());
    assert!(!jules.join("roles/planners/specifier/notes").exists());
    assert!(!jules.join("roles/planners/specifier/feedbacks").exists());
    assert!(!jules.join("roles/planners/specifier/role.yml").exists());
    assert!(!jules.join("roles/implementers/executor/notes").exists());
    assert!(!jules.join("roles/implementers/executor/feedbacks").exists());
    assert!(!jules.join("roles/implementers/executor/role.yml").exists());
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
fn template_creates_implementer_without_notes() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli().args(["template", "-l", "implementers", "-n", "custom-impl"]).assert().success();

    // Implementer roles should NOT have notes, feedbacks, or role.yml directories
    let role_path = ctx.jules_path().join("roles/implementers/custom-impl");
    let notes_path = role_path.join("notes");
    let feedbacks_path = role_path.join("feedbacks");
    let role_yml = role_path.join("role.yml");
    assert!(!notes_path.exists(), "Implementer role should not have notes directory");
    assert!(!feedbacks_path.exists(), "Implementer role should not have feedbacks directory");
    assert!(
        !role_yml.exists(),
        "Implementer role should not have role.yml (behavior defined in archetype)"
    );
}
