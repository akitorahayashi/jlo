//! Integration tests validating that mock output files comply with doctor schema validation.
//!
//! These tests ensure that pre-defined mock files in src/testing/assets/mock/
//! pass doctor validation, preventing schema drift and workflow failures.

mod common;

use common::TestContext;
use std::fs;

/// Helper to initialize scaffold with flat exchange structure
fn setup_scaffold(ctx: &TestContext) {
    ctx.cli().args(["init", "--remote"]).assert().success();
    ctx.cli().args(["workflow", "bootstrap"]).assert().success();
}

#[test]
fn mock_narrator_change_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    // Copy mock change file to workspace
    let mock_change = include_str!("../src/assets/mock/narrator_change.yml");
    let exchange_dir = ctx.jules_path().join("exchange");
    fs::create_dir_all(&exchange_dir).expect("Failed to create exchange directory");

    let changes_file = exchange_dir.join("changes.yml");
    fs::write(&changes_file, mock_change).expect("Failed to write changes file");

    // Run doctor to validate
    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_observer_event_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    // Copy mock event file to workspace
    let mock_event = include_str!("../src/assets/mock/observer_event.yml");
    let events_dir = ctx.jules_path().join("exchange").join("events").join("pending");

    fs::create_dir_all(&events_dir).expect("Failed to create events directory");

    let event_file = events_dir.join("mock01.yml");
    fs::write(&event_file, mock_event).expect("Failed to write event file");

    // Run doctor to validate
    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_decider_issue_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    // Create the referenced event in decided state (simulating what decider does)
    let mock_event = include_str!("../src/assets/mock/observer_event.yml");
    let events_dir = ctx.jules_path().join("exchange").join("events").join("decided");
    fs::create_dir_all(&events_dir).expect("Failed to create events directory");

    // Event must have issue_id set when in decided state
    let impl_event_id = "evt001"; // 6 lowercase alphanumeric chars
    let impl_issue_id = "iss001"; // 6 lowercase alphanumeric chars
    let impl_event_file = events_dir.join(format!("{}.yml", impl_event_id));
    fs::write(
        &impl_event_file,
        mock_event
            .replace("mock01", impl_event_id)
            .replace("issue_id: \"\"", &format!("issue_id: \"{}\"", impl_issue_id)),
    )
    .expect("Failed to write event file");

    let planner_event_id = "evt002"; // 6 lowercase alphanumeric chars
    let planner_issue_id = "pln001"; // 6 lowercase alphanumeric chars
    let planner_event_file = events_dir.join(format!("{}.yml", planner_event_id));
    fs::write(
        &planner_event_file,
        mock_event
            .replace("mock01", planner_event_id)
            .replace("issue_id: \"\"", &format!("issue_id: \"{}\"", planner_issue_id)),
    )
    .expect("Failed to write planner event file");

    // Copy mock requirement file to workspace - apply the same transformations as mock decider
    let mock_requirement = include_str!("../src/assets/mock/decider_requirement.yml");
    let requirements_dir = ctx.jules_path().join("exchange").join("requirements");

    fs::create_dir_all(&requirements_dir).expect("Failed to create requirements directory");

    // Test 1: Requirement without deep analysis (implementer-ready)
    let impl_issue_file = requirements_dir.join("impl-issue.yml");
    fs::write(
        &impl_issue_file,
        mock_requirement
            .replace("mock01", impl_issue_id)
            .replace("event1", impl_event_id),
    )
    .expect("Failed to write impl requirement file");

    // Test 2: Requirement with deep analysis (planner-ready) - must include deep_analysis_reason
    let planner_issue_file = requirements_dir.join("planner-issue.yml");
    fs::write(
        &planner_issue_file,
        mock_requirement
            .replace("mock01", planner_issue_id)
            .replace("event1", planner_event_id)
            .replace(
                "requires_deep_analysis: false",
                "requires_deep_analysis: true\ndeep_analysis_reason: \"Mock issue requires architectural analysis\"",
            ),
    )
    .expect("Failed to write planner issue file");

    // Run doctor to validate
    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_observer_comment_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    // Create innovator persona directory with a comment
    let comments_dir =
        ctx.jules_path().join("exchange").join("innovators").join("alice").join("comments");

    fs::create_dir_all(&comments_dir).expect("Failed to create comments directory");

    let mock_comment = include_str!("../src/assets/mock/observer_comment.yml");
    // Replace template placeholders so doctor won't flag them
    let comment_content = mock_comment
        .replace("mock-author", "taxonomy")
        .replace("test-tag", "mock-local-20260205120000");

    let comment_file = comments_dir.join("observer-taxonomy-abc123.yml");
    fs::write(&comment_file, comment_content).expect("Failed to write comment file");

    // Run doctor to validate
    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_innovator_idea_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let room_dir = ctx.jules_path().join("exchange").join("innovators").join("alice");
    let comments_dir = room_dir.join("comments");
    fs::create_dir_all(&comments_dir).expect("Failed to create innovator comments directory");

    // Seed perspective so the room resembles real execution context.
    let perspective = r#"schema_version: 1
persona: "alice"
focus: "High-leverage improvements"
current_view: |
  Current architecture has repetitive workflow logic.
historical_learnings: |
  Role-level contracts reduce drift when coupled with doctor checks.
recent_proposals: []
"#;
    fs::write(room_dir.join("perspective.yml"), perspective).expect("Failed to write perspective");

    let mock_idea = include_str!("../src/assets/mock/innovator_idea.yml");
    let idea = mock_idea
        .replace("mock01", "abc123")
        .replace("mock-persona", "alice")
        .replace("test-tag", "mock-local-20260205120000");
    fs::write(room_dir.join("idea.yml"), idea).expect("Failed to write idea");

    ctx.cli().args(["doctor"]).assert().success();
}
