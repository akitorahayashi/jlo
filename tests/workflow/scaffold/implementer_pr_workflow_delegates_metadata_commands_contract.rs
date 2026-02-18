use crate::harness::TestContext;
use std::fs;

#[test]
fn implementer_metadata_path_delegates_to_unified_process_command() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-implementer-pr.yml")).unwrap();

    assert!(workflow.contains("process-implementer-pr-metadata:"));
    assert!(workflow.contains("'jules-implementer-*'"));
    assert!(workflow.contains("jlo workflow process-pr metadata"));
    assert!(workflow.contains("--fail-on-error"));
    assert!(workflow.contains("secrets.JULES_LINKED_GH_PAT"));
    assert!(workflow.contains("secrets.JLO_BOT_TOKEN"));
    assert!(!workflow.contains("--mode metadata"));

    assert!(
        !fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml"))
            .unwrap()
            .contains("process-implementer-pr-metadata:"),
        "scheduled workflow should not inline metadata job"
    );
    assert!(
        !root.join(".github/workflows/jules-implementer-label.yml").exists(),
        "legacy implementer-label workflow should not be installed"
    );
    assert!(
        !root.join(".github/workflows/jules-pr-summary-request.yml").exists(),
        "legacy summary-request workflow should not be installed"
    );
}
