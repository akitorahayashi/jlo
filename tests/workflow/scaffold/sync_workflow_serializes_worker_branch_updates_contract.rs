use crate::harness::TestContext;
use std::fs;

#[test]
fn sync_workflow_serializes_worker_branch_updates() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let sync = fs::read_to_string(root.join(".github/workflows/jules-sync.yml")).unwrap();

    assert!(sync.contains("\n    concurrency:\n"));
    assert!(sync.contains("group: 'jules-sync-"));
    assert!(sync.contains("cancel-in-progress: false"));
}
