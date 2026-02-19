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
    assert!(!workflow.contains("workflow_call:"), "Should not support workflow_call trigger");
    assert!(workflow.contains("MOCK_MODE:"), "Should set MOCK_MODE env var");
    assert!(workflow.contains("JULES_MOCK_TAG:"), "Should set JULES_MOCK_TAG env var");
    assert!(workflow.contains("JLO_RUN_FLAGS:"), "Should set JLO_RUN_FLAGS env var");
    assert!(
        workflow.contains("cleanup-mock-artifacts:"),
        "Should include integrated mock cleanup job"
    );
    assert!(
        workflow.contains("needs: [publish-proposals, wait-after-planner, wait-after-implementer]"),
        "Cleanup must wait for publish-proposals to avoid issue-close race"
    );
    assert!(
        workflow.contains(
            "github.event_name == 'workflow_dispatch' && github.event.inputs.mock == 'true'"
        ),
        "Cleanup job should gate on workflow_dispatch mock input"
    );
    assert!(!workflow.contains("inputs.mock == true"));
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
        workflow.contains("jlo workflow doctor"),
        "Cleanup flow should validate with doctor as an explicit workflow step"
    );
    assert!(
        workflow.contains("jlo workflow push worker-branch"),
        "Cleanup flow should publish worker updates via workflow push worker-branch"
    );
    assert!(workflow.contains("run-innovators:"), "Should have integrated innovators job");
    assert!(
        workflow.contains("run-innovators:\n    needs: [\"bootstrap\"]"),
        "run-innovators should start after bootstrap only"
    );
    assert!(
        !workflow.contains("run-innovators:\n    needs: [\"resolve-run-plan\"]"),
        "run-innovators must not wait for resolve-run-plan"
    );
    assert!(
        workflow.contains("jlo workflow run innovators"),
        "Scheduled workflow should run innovators directly"
    );
    assert!(
        workflow.contains("\n          - innovators\n"),
        "Entry point choices should include innovators"
    );
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
        workflow.contains("observers)\n              run_observers=true\n              run_decider=true\n              run_planner=true\n              run_implementer=true"),
        "Observers entry-point should continue downstream without innovators"
    );
    assert!(
        workflow.contains("(github.event.inputs.entry_point || 'narrator') == 'innovators'"),
        "Integrated run-innovators job should run only for innovators entry-point in dispatch"
    );
    assert!(
        workflow.contains("github.event_name == 'schedule' ||"),
        "run-innovators should always run on schedule runs"
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
    assert!(workflow.contains("wait-after-narrator:"), "Should include narrator-specific wait job");
    assert!(
        !workflow.contains("wait-after-initial-requests:"),
        "Legacy wait-after-initial-requests job should not exist"
    );
    assert!(
        workflow.contains("needs: [\"resolve-run-plan\", \"wait-after-narrator\"]"),
        "run-observers should depend on narrator wait only when narrator path is used"
    );
    assert!(
        workflow.contains("fromJSON(needs.resolve-run-plan.outputs.json).run_observers == true &&\n            needs.wait-after-narrator.result == 'success'"
        ),
        "run-observers should stay simple and require narrator-wait gate success"
    );
    assert!(
        workflow.contains("number_of_api_requests_succeeded > 0"),
        "Wait jobs should gate on output-driven number_of_api_requests_succeeded"
    );
    assert!(
        !workflow.contains("generate-decider-matrix"),
        "Removed generate-decider-matrix job should not exist"
    );
    assert!(
        !workflow.contains("generate-routing-matrix"),
        "Removed generate-routing-matrix job should not exist"
    );
    assert!(
        workflow.contains("Narrator not requested; skipping wait."),
        "wait-after-narrator should explicitly skip waiting when narrator is not requested"
    );
    assert!(
        workflow.contains("fromJSON(needs.resolve-run-plan.outputs.json).run_narrator == false &&"),
        "wait-after-narrator should branch for non-narrator entry points"
    );
}
