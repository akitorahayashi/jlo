use crate::harness::TestContext;
use std::fs;

#[test]
fn installed_workflow_scaffold_includes_mock_support() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml")).unwrap();

    assert!(workflow.contains("mock:"), "Should have mock input");
    assert!(workflow.contains("workflow_call:"), "Should support workflow_call trigger");
    assert!(workflow.contains("MOCK_MODE:"), "Should set MOCK_MODE env var");
    assert!(workflow.contains("JULES_MOCK_TAG:"), "Should set JULES_MOCK_TAG env var");
    assert!(workflow.contains("JLO_RUN_FLAGS:"), "Should set JLO_RUN_FLAGS env var");
    assert!(
        workflow.contains("cleanup-mock-artifacts:"),
        "Should include integrated mock cleanup job"
    );
    assert!(
        workflow.contains(
            "github.event_name == 'workflow_dispatch' && github.event.inputs.mock == 'true'"
        ),
        "Cleanup job should gate on workflow_dispatch mock input"
    );
    assert!(
        workflow.contains("github.event_name == 'workflow_call' && inputs.mock == true"),
        "Cleanup job should gate on workflow_call mock input"
    );
    assert!(
        !workflow.contains("Skip when not mock mode"),
        "Cleanup gating should be handled at job-level condition"
    );
    assert!(
        !workflow.contains("JULES_MOCK_TAG: ${{ env.JULES_MOCK_TAG }}"),
        "Job-level env should not re-export JULES_MOCK_TAG from env context"
    );
    assert!(
        !root.join(".github/workflows/jules-mock-cleanup.yml").exists(),
        "Should not install separate mock cleanup workflow"
    );
    assert!(
        workflow.contains("Cleanup uses a PR merge path to satisfy branch protection"),
        "Cleanup flow should explain PR-based branch protection rationale"
    );
    assert!(
        workflow.contains("authority is centralized in jules-automerge workflow"),
        "Cleanup flow should explain centralized auto-merge ownership"
    );
    assert!(workflow.contains("run-innovators-1:"), "Should have first innovator pass job");
    assert!(workflow.contains("run-innovators-2:"), "Should have second innovator pass job");
    assert!(workflow.contains("publish-proposals:"), "Should have publish-proposals job");
    assert!(!workflow.contains("- innovators"), "Entry point choices should exclude innovators");
    assert!(
        workflow.contains("\n          - requirements\n"),
        "Entry point choices should include requirements routing start-point"
    );
    assert!(
        !workflow.contains("\n          - planner\n"),
        "Entry point choices should exclude planner"
    );
    assert!(
        !workflow.contains("\n          - implementer\n"),
        "Entry point choices should exclude implementer"
    );
    assert!(
        workflow.contains("observers)\n              run_innovators_1=true"),
        "Observers entry-point should trigger innovators"
    );
    assert!(
        workflow.contains("observers)\n              run_innovators_1=true\n              run_observers=true\n              run_innovators_2=true\n              run_publish_proposals=true\n              run_decider=true\n              run_planner=true\n              run_implementer=true"),
        "Observers entry-point should continue downstream through decider/planner/implementer"
    );
    assert!(
        workflow.contains("decider)\n              run_decider=true"),
        "Decider entry-point should start at decider"
    );
    assert!(
        workflow.contains("decider)\n              run_decider=true\n              run_planner=true\n              run_implementer=true"),
        "Decider entry-point should continue to planner/implementer without innovators"
    );
    assert!(
        workflow.contains(
            "requirements)\n              run_planner=true\n              run_implementer=true"
        ),
        "Requirements entry-point should start from planner/implementer routing"
    );
    assert!(
        workflow.contains("always() &&\n      needs.check-schedule.result == 'success'"),
        "wait-after-initial-requests should use always() to handle skipped upstream jobs"
    );
}
