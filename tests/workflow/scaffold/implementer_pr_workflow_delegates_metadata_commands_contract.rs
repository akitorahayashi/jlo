use crate::harness::TestContext;
use std::fs;

#[test]
fn implementer_pr_workflow_delegates_metadata_commands() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-implementer-pr.yml"))
        .expect("read implementer PR workflow");

    assert!(workflow.contains("branches:"));
    assert!(workflow.contains("'jules-implementer-*'"));
    assert!(workflow.contains("jlo workflow gh pr sync-category-label"));
    assert!(workflow.contains("jlo workflow gh pr comment-summary-request"));
    assert!(workflow.contains("secrets.JLO_BOT_TOKEN"));
    assert!(workflow.contains("secrets.JULES_LINKED_GH_TOKEN"));

    assert!(
        !root.join(".github/workflows/jules-implementer-label.yml").exists(),
        "legacy implementer-label workflow should not be installed"
    );
    assert!(
        !root.join(".github/workflows/jules-pr-summary-request.yml").exists(),
        "legacy summary-request workflow should not be installed"
    );
}
