//! Integration tests validating that mock output files comply with doctor schema validation.
//!
//! These tests ensure that pre-defined mock files in src/testing/assets/mock/
//! pass doctor validation, preventing schema drift and workflow failures.

mod common;

use common::TestContext;
use std::fs;

/// Helper to initialize scaffold and create a workstream
fn setup_scaffold_with_workstream(ctx: &TestContext, workstream: &str) {
    ctx.cli().args(["init", "scaffold"]).assert().success();

    // Create a workstream with scheduled.toml
    let workstream_path = ctx.jules_path().join("workstreams").join(workstream);
    fs::create_dir_all(&workstream_path).expect("Failed to create workstream dir");

    let scheduled_toml = workstream_path.join("scheduled.toml");
    fs::write(
        scheduled_toml,
        r#"version = 1
enabled = false

[observers]
roles = []

[deciders]
roles = []
"#,
    )
    .expect("Failed to write scheduled.toml");

    // Create expected directory structure
    let exchange = workstream_path.join("exchange");
    fs::create_dir_all(exchange.join("events").join("pending")).unwrap();
    fs::create_dir_all(exchange.join("events").join("decided")).unwrap();
    fs::create_dir_all(exchange.join("issues").join("bugs")).unwrap();
    fs::create_dir_all(exchange.join("issues").join("docs")).unwrap();
    fs::create_dir_all(exchange.join("issues").join("feats")).unwrap();
    fs::create_dir_all(exchange.join("issues").join("refacts")).unwrap();
    fs::create_dir_all(exchange.join("issues").join("tests")).unwrap();
    fs::create_dir_all(workstream_path.join("workstations")).unwrap();
}

#[test]
fn mock_narrator_change_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold_with_workstream(&ctx, "generic");

    // Copy mock change file to workspace
    let mock_change = include_str!("../src/assets/mock/narrator_change.yml");
    let changes_dir = ctx.jules_path().join("changes");
    fs::create_dir_all(&changes_dir).expect("Failed to create changes directory");

    let changes_file = changes_dir.join("latest.yml");
    fs::write(&changes_file, mock_change).expect("Failed to write changes file");

    // Run doctor to validate
    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_observer_event_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold_with_workstream(&ctx, "test-workstream");

    // Copy mock event file to workspace
    let mock_event = include_str!("../src/assets/mock/observer_event.yml");
    let events_dir = ctx
        .jules_path()
        .join("workstreams")
        .join("test-workstream")
        .join("exchange")
        .join("events")
        .join("pending");

    fs::create_dir_all(&events_dir).expect("Failed to create events directory");

    let event_file = events_dir.join("mock01.yml");
    fs::write(&event_file, mock_event).expect("Failed to write event file");

    // Run doctor to validate
    ctx.cli().args(["doctor", "--workstream", "test-workstream"]).assert().success();
}

#[test]
fn mock_decider_issue_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold_with_workstream(&ctx, "test-workstream");

    // Create the referenced event in decided state (simulating what decider does)
    let mock_event = include_str!("../src/assets/mock/observer_event.yml");
    let events_dir = ctx
        .jules_path()
        .join("workstreams")
        .join("test-workstream")
        .join("exchange")
        .join("events")
        .join("decided");
    fs::create_dir_all(&events_dir).expect("Failed to create events directory");

    // Event must have issue_id set when in decided state
    let event_id = "evt001"; // 6 lowercase alphanumeric chars
    let issue_id = "iss001"; // 6 lowercase alphanumeric chars
    let event_file = events_dir.join(format!("{}.yml", event_id));
    fs::write(
        &event_file,
        mock_event
            .replace("mock01", event_id)
            .replace("issue_id: \"\"", &format!("issue_id: \"{}\"", issue_id)),
    )
    .expect("Failed to write event file");

    // Copy mock issue file to workspace - apply the same transformations as mock decider
    let mock_issue = include_str!("../src/assets/mock/decider_issue.yml");
    let issues_dir = ctx
        .jules_path()
        .join("workstreams")
        .join("test-workstream")
        .join("exchange")
        .join("issues")
        .join("bugs");

    fs::create_dir_all(&issues_dir).expect("Failed to create issues directory");

    // Test 1: Issue without deep analysis (implementer-ready)
    let impl_issue_file = issues_dir.join("impl-issue.yml");
    fs::write(
        &impl_issue_file,
        mock_issue.replace("mock01", issue_id).replace("event1", event_id), // Replace source_events placeholder
    )
    .expect("Failed to write impl issue file");

    // Test 2: Issue with deep analysis (planner-ready) - must include deep_analysis_reason
    let planner_issue_file = issues_dir.join("planner-issue.yml");
    fs::write(
        &planner_issue_file,
        mock_issue
            .replace("mock01", "pln001")
            .replace("event1", event_id)
            .replace(
                "requires_deep_analysis: false",
                "requires_deep_analysis: true\ndeep_analysis_reason: \"Mock issue requires architectural analysis\"",
            ),
    )
    .expect("Failed to write planner issue file");

    // Run doctor to validate
    ctx.cli().args(["doctor", "--workstream", "test-workstream"]).assert().success();
}
