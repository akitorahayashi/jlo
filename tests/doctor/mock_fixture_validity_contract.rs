use crate::harness::TestContext;
use std::fs;

fn setup_scaffold(ctx: &TestContext) {
    ctx.init_remote_and_bootstrap();
    fs::write(ctx.jules_path().join("JULES.md"), "# Jules\n").expect("write JULES.md");
    fs::write(ctx.jules_path().join("README.md"), "# Workspace\n").expect("write README.md");
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
fn mock_decider_requirement_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let mock_event = include_str!("../../src/assets/mock/observer_event.yml");
    let events_dir = ctx.jules_path().join("exchange/events/decided");
    fs::create_dir_all(&events_dir).expect("Failed to create events directory");

    let impl_event_id = "evt001";
    let impl_requirement_id = "req001";
    fs::write(
        events_dir.join(format!("{}.yml", impl_event_id)),
        mock_event.replace("mock01", impl_event_id).replace(
            "requirement_id: \"\"",
            &format!("requirement_id: \"{}\"", impl_requirement_id),
        ),
    )
    .expect("Failed to write event file");

    let planner_event_id = "evt002";
    let planner_requirement_id = "pln001";
    fs::write(
        events_dir.join(format!("{}.yml", planner_event_id)),
        mock_event.replace("mock01", planner_event_id).replace(
            "requirement_id: \"\"",
            &format!("requirement_id: \"{}\"", planner_requirement_id),
        ),
    )
    .expect("Failed to write planner event file");

    let mock_requirement = include_str!("../../src/assets/mock/decider_requirement.yml");
    let requirements_dir = ctx.jules_path().join("exchange/requirements");
    fs::create_dir_all(&requirements_dir).expect("Failed to create requirements directory");

    fs::write(
        requirements_dir.join("impl-requirement.yml"),
        mock_requirement.replace("mock01", impl_requirement_id).replace("event1", impl_event_id),
    )
    .expect("Failed to write impl requirement file");

    fs::write(
        requirements_dir.join("planner-requirement.yml"),
        mock_requirement
            .replace("mock01", planner_requirement_id)
            .replace("event1", planner_event_id)
            .replace(
                "implementation_ready: true\nplanner_request_reason: \"\"",
                "implementation_ready: false\nplanner_request_reason: \"Mock requirement requires architectural analysis\"",
            ),
    )
    .expect("Failed to write planner requirement file");

    ctx.cli().args(["doctor"]).assert().success();
}

#[test]
fn mock_innovator_proposal_file_passes_doctor() {
    let ctx = TestContext::new();
    setup_scaffold(&ctx);

    let workstation_dir = ctx.jules_path().join("workstations/alice");
    fs::create_dir_all(&workstation_dir).expect("Failed to create workstation directory");

    let perspective = r#"schema_version: 1
role: "alice"
focus: "High-leverage improvements"
repository_observations:
  codebase_state:
    - "Current architecture has repetitive workflow logic."
  startup_and_runtime_contracts:
    - "Bootstrap owns workstation lifecycle."
  decision_quality_gaps:
    - "Proposal alternatives are currently sparse."
  leverage_candidates:
    - "Generate three proposals per run."
thinking_notes:
  hypotheses:
    - "Parallel ideation increases option quality."
  tradeoff_assessment:
    - "Broader option sets increase evaluation load."
  rejected_paths:
    - "Single-proposal-only execution."
feedback_assimilation:
  observer_inputs: []
  next_focus:
    - "Track proposal quality through issue outcomes."
recent_proposals:
  - "Improve workflow error messages"
"#;
    fs::write(workstation_dir.join("perspective.yml"), perspective)
        .expect("Failed to write perspective");

    let proposals_dir = ctx.jules_path().join("exchange/proposals");
    fs::create_dir_all(&proposals_dir).expect("Failed to create proposals directory");
    let proposal = r#"schema_version: 1
id: "abc123"
role: "alice"
created_at: "2026-02-05"
title: "Improve workflow error messages"
problem: |
  Error context is currently too terse.
introduction: |
  Introduce richer workflow error narratives with actionable hints.
importance: |
  Faster diagnosis improves iteration speed.
impact_surface:
  - "workflow"
implementation_cost: "medium"
consistency_risks:
  - "Mixed error formats during rollout"
verification_signals:
  - "Fewer reruns due to ambiguous failures"
"#;
    fs::write(proposals_dir.join("alice-improve-workflow-error-messages.yml"), proposal)
        .expect("Failed to write proposal");

    ctx.cli().args(["doctor"]).assert().success();
}
