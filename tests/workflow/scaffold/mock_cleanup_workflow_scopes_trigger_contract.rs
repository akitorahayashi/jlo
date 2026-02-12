use crate::harness::TestContext;
use std::fs;

#[test]
fn mock_cleanup_workflow_runs_only_for_dispatch_or_call_on_target_branch() {
    let ctx = TestContext::new();
    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-mock-cleanup.yml")).unwrap();

    assert!(workflow.contains("workflow_run:"));
    assert!(workflow.contains("branches:"));
    assert!(workflow.contains("- 'main'"));
    assert!(workflow.contains("github.event.workflow_run.event == 'workflow_dispatch'"));
    assert!(workflow.contains("github.event.workflow_run.event == 'workflow_call'"));
}
