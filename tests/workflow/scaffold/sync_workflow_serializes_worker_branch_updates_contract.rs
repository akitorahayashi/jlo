use crate::harness::TestContext;
use std::fs;

#[test]
fn sync_path_serializes_worker_branch_updates_within_primary_workflow() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml")).unwrap();

    assert!(workflow.contains("bootstrap:"));
    assert!(workflow.contains("Sync worker branch"));
    assert!(workflow.contains("jlo workflow bootstrap worker-branch"));
    assert!(workflow.contains("jlo workflow gh push worker-branch"));
    assert!(!workflow.contains("git push origin \"${JULES_WORKER_BRANCH}\""));
    assert!(!workflow.contains("sync-worker-branch:"));

    assert!(
        !root.join(".github/workflows/jules-sync.yml").exists(),
        "standalone sync workflow should not be installed"
    );
}
