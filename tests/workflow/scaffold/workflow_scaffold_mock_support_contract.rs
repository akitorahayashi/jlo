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
    assert!(workflow.contains("run-innovators-1:"), "Should have first innovator pass job");
    assert!(workflow.contains("run-innovators-2:"), "Should have second innovator pass job");
    assert!(workflow.contains("publish-proposals:"), "Should have publish-proposals job");
    assert!(workflow.contains("- innovators"), "Entry point choices should include innovators");
}
