use crate::harness::TestContext;
use std::fs;

fn setup_scaffold(ctx: &TestContext) {
    ctx.init_remote_and_bootstrap();
}

#[test]
fn mock_narrator_change_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let mock_change = include_str!("../../src/assets/mock/narrator_change.yml");
    let exchange_dir = ctx.jules_path().join("exchange");
    fs::create_dir_all(&exchange_dir).expect("Failed to create exchange directory");

    fs::write(exchange_dir.join("changes.yml"), mock_change).expect("Failed to write changes file");

    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_observer_event_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let mock_event = include_str!("../../src/assets/mock/observer_event.yml");
    let events_dir = ctx.jules_path().join("exchange/events/pending");
    fs::create_dir_all(&events_dir).expect("Failed to create events directory");

    fs::write(events_dir.join("mock01.yml"), mock_event).expect("Failed to write event file");

    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_decider_issue_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let mock_event = include_str!("../../src/assets/mock/observer_event.yml");
    let events_dir = ctx.jules_path().join("exchange/events/decided");
    fs::create_dir_all(&events_dir).expect("Failed to create events directory");

    let impl_event_id = "evt001";
    let impl_issue_id = "iss001";
    fs::write(
        events_dir.join(format!("{}.yml", impl_event_id)),
        mock_event
            .replace("mock01", impl_event_id)
            .replace("issue_id: \"\"", &format!("issue_id: \"{}\"", impl_issue_id)),
    )
    .expect("Failed to write event file");

    let planner_event_id = "evt002";
    let planner_issue_id = "pln001";
    fs::write(
        events_dir.join(format!("{}.yml", planner_event_id)),
        mock_event
            .replace("mock01", planner_event_id)
            .replace("issue_id: \"\"", &format!("issue_id: \"{}\"", planner_issue_id)),
    )
    .expect("Failed to write planner event file");

    let mock_requirement = include_str!("../../src/assets/mock/decider_requirement.yml");
    let requirements_dir = ctx.jules_path().join("exchange/requirements");
    fs::create_dir_all(&requirements_dir).expect("Failed to create requirements directory");

    fs::write(
        requirements_dir.join("impl-issue.yml"),
        mock_requirement.replace("mock01", impl_issue_id).replace("event1", impl_event_id),
    )
    .expect("Failed to write impl requirement file");

    fs::write(
        requirements_dir.join("planner-issue.yml"),
        mock_requirement
            .replace("mock01", planner_issue_id)
            .replace("event1", planner_event_id)
            .replace(
                "requires_deep_analysis: false",
                "requires_deep_analysis: true\ndeep_analysis_reason: \"Mock issue requires architectural analysis\"",
            ),
    )
    .expect("Failed to write planner issue file");

    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_observer_comment_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let comments_dir = ctx.jules_path().join("exchange/innovators/alice/comments");
    fs::create_dir_all(&comments_dir).expect("Failed to create comments directory");

    let mock_comment = include_str!("../../src/assets/mock/observer_comment.yml");
    let comment_content = mock_comment
        .replace("mock-author", "taxonomy")
        .replace("test-tag", "mock-local-20260205120000");

    fs::write(comments_dir.join("observer-taxonomy-abc123.yml"), comment_content)
        .expect("Failed to write comment file");

    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_innovator_idea_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let room_dir = ctx.jules_path().join("exchange/innovators/alice");
    fs::create_dir_all(room_dir.join("comments"))
        .expect("Failed to create innovator comments directory");

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

    let mock_idea = include_str!("../../src/assets/mock/innovator_idea.yml");
    let idea = mock_idea
        .replace("mock01", "abc123")
        .replace("mock-persona", "alice")
        .replace("test-tag", "mock-local-20260205120000");
    fs::write(room_dir.join("idea.yml"), idea).expect("Failed to write idea");

    ctx.cli().args(["doctor"]).assert().success();
}
