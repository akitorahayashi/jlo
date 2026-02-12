use crate::harness::TestContext;
use std::fs;

#[test]
fn sync_path_serializes_worker_branch_updates_within_primary_workflow() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml")).unwrap();

    assert!(workflow.contains("sync-worker-branch:"));
    assert!(workflow.contains("group: 'jules-sync-"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("Merge target branch into worker"));

    assert!(
        !root.join(".github/workflows/jules-sync.yml").exists(),
        "standalone sync workflow should not be installed"
    );
}
