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
        .args(["template", "-l", "observers", "-n", "security", "-w", "generic"])
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
    ctx.cli()
        .args(["tp", "-l", "deciders", "-n", "my-decider", "-w", "generic"])
        .assert()
        .success();

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
    ctx.assert_changes_directory_exists();
    ctx.assert_narrator_exists();

    // Verify multi-role layers have role.yml under roles/ container and schemas/ directories
    let jules = ctx.jules_path();
    assert!(jules.join("roles/observers/roles/taxonomy/role.yml").exists());
    assert!(jules.join("roles/observers/roles/data_arch/role.yml").exists());
    assert!(jules.join("roles/observers/roles/qa/role.yml").exists());
    assert!(jules.join("roles/observers/roles/cov/role.yml").exists());
    assert!(jules.join("roles/observers/roles/consistency/role.yml").exists());
    assert!(jules.join("roles/observers/schemas").exists());
    assert!(jules.join("roles/observers/prompt_assembly.yml").exists());

    // Deciders have role.yml under roles/ container
    assert!(jules.join("roles/deciders/roles/triage_generic/role.yml").exists());
    assert!(jules.join("roles/deciders/schemas").exists());
    assert!(jules.join("roles/deciders/prompt_assembly.yml").exists());

    // Single-role layers have flat structure (no roles subdirectory)
    assert!(jules.join("roles/narrators/prompt.yml").exists());
    assert!(jules.join("roles/narrators/contracts.yml").exists());
    assert!(jules.join("roles/narrators/schemas/change.yml").exists());
    assert!(jules.join("roles/planners/prompt.yml").exists());
    assert!(jules.join("roles/planners/contracts.yml").exists());
    assert!(jules.join("roles/implementers/prompt.yml").exists());
    assert!(jules.join("roles/implementers/contracts.yml").exists());
}

#[test]
#[serial]
fn template_creates_observer_role() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    ctx.cli()
        .args(["template", "-l", "observers", "-n", "custom-obs", "-w", "generic"])
        .assert()
        .success();

    // Observer roles should have role.yml under roles/ container
    let role_path = ctx.jules_path().join("roles/observers/roles/custom-obs");
    let role_yml = role_path.join("role.yml");
    assert!(role_yml.exists(), "Observer role should have role.yml");
}

#[test]
#[serial]
fn template_rejects_single_role_layers() {
    let ctx = TestContext::new();

    ctx.cli().arg("init").assert().success();

    // Narrator is single-role and should not accept template creation
    ctx.cli()
        .args(["template", "-l", "narrators", "-n", "custom-narrator"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-role"));

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
