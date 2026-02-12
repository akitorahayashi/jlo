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
    assert!(workflow.contains("run-innovators-1:"), "Should have first innovator pass job");
    assert!(workflow.contains("run-innovators-2:"), "Should have second innovator pass job");
    assert!(workflow.contains("publish-proposals:"), "Should have publish-proposals job");
    assert!(workflow.contains("- innovators"), "Entry point choices should include innovators");
}
