use crate::harness::TestContext;
use std::fs;

#[test]
fn integrator_workflow_does_not_require_runner_side_linked_pat() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-integrator.yml")).unwrap();

    assert!(!workflow.contains("secrets.JULES_LINKED_GH_PAT"));
    assert!(!workflow.contains("JULES_LINKED_GH_PAT is required for integrator"));
    assert!(workflow.contains("JULES_API_KEY"));
    assert!(workflow.contains("secrets.JLO_BOT_TOKEN"));
}
