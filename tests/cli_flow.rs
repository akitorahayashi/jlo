mod common;

use common::TestContext;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn user_can_init_and_create_custom_role() {
    let ctx = TestContext::new();

    // Initialize workspace
    ctx.cli().args(["init", "--remote"]).assert().success();

    // Create a custom observer role via create command (writes to .jlo/)
    ctx.cli()
        .args(["create", "observers", "security"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created new"));

    let role_path = ctx.jlo_path().join("roles/observers/security/role.yml");
    assert!(role_path.exists(), "Custom role should exist in .jlo/ control plane");
}

#[test]
#[serial]
fn user_can_use_command_aliases() {
    let ctx = TestContext::new();

    // Use 'i' alias for init
    ctx.cli().args(["i", "--remote"]).assert().success();

    // Use 'cr' alias for create
    ctx.cli().args(["cr", "observers", "my-observer"]).assert().success();

    let role_path = ctx.jlo_path().join("roles/observers/my-observer/role.yml");
    assert!(role_path.exists(), "Role created via alias should exist in .jlo/");
}

#[test]
#[serial]
fn init_creates_complete_layer_structure() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();

    // Verify layer structure
    ctx.assert_jules_exists();
    ctx.assert_layer_structure_exists();
    ctx.assert_events_structure_exists();
    ctx.assert_issues_directory_exists();
    ctx.assert_default_scheduled_roles_exist();
    ctx.assert_narrator_exists();

    // Verify multi-role layers have role.yml under roles/ container and schemas/ directories
    let jules = ctx.jules_path();
    let jlo = ctx.jlo_path();
    assert!(jlo.join("roles/observers/taxonomy/role.yml").exists());
    assert!(jlo.join("roles/observers/data_arch/role.yml").exists());
    assert!(jlo.join("roles/observers/structural_arch/role.yml").exists());
    assert!(jlo.join("roles/observers/qa/role.yml").exists());
    assert!(jlo.join("roles/observers/cov/role.yml").exists());
    assert!(jlo.join("roles/observers/consistency/role.yml").exists());
    assert!(jlo.join("roles/innovators/recruiter/role.yml").exists());
    assert!(jules.join("roles/observers/schemas").exists());
    assert!(jules.join("roles/observers/prompt_assembly.j2").exists());

    // Decider is single-role and runtime-driven from contracts only (no .jlo role.yml)
    assert!(!jlo.join("roles/deciders/role.yml").exists());
    assert!(jules.join("roles/deciders/schemas").exists());
    assert!(jules.join("roles/deciders/prompt_assembly.j2").exists());

    // Single-role layers have flat structure (no roles subdirectory)
    assert!(jules.join("roles/narrator/contracts.yml").exists());
    assert!(jules.join("roles/narrator/schemas/changes.yml").exists());
    assert!(jules.join("roles/narrator/tasks/bootstrap_summary.yml").exists());
    assert!(jules.join("roles/narrator/tasks/overwrite_summary.yml").exists());
    assert!(jules.join("roles/planners/contracts.yml").exists());
    assert!(jules.join("roles/implementers/contracts.yml").exists());

    // Innovators use phase-specific contracts
    assert!(jules.join("roles/innovators/contracts_creation.yml").exists());
    assert!(jules.join("roles/innovators/contracts_refinement.yml").exists());

    // All layers have tasks/ directory
    for layer in ["narrator", "observers", "deciders", "planners", "implementers", "innovators"] {
        assert!(
            jules.join("roles").join(layer).join("tasks").exists(),
            "Layer {} should have tasks/ directory",
            layer
        );
    }
}

#[test]
#[serial]
fn create_role_in_observers() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    ctx.cli().args(["create", "observers", "custom-obs"]).assert().success();

    // Role should exist in .jlo/ control plane
    let role_path = ctx.jlo_path().join("roles/observers/custom-obs/role.yml");
    assert!(role_path.exists(), "Observer role should have role.yml in .jlo/");
}

#[test]
#[serial]
fn create_role_rejects_single_role_layers() {
    let ctx = TestContext::new();

    ctx.cli().args(["init", "--remote"]).assert().success();

    // Narrator is single-role and should not accept role creation
    ctx.cli()
        .args(["create", "narrator", "custom-narrator"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-role"));

    // Planners are single-role and should not accept role creation
    ctx.cli()
        .args(["create", "planners", "custom-planner"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-role"));

    // Implementers are single-role and should not accept role creation
    ctx.cli()
        .args(["create", "implementers", "custom-impl"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-role"));
}
