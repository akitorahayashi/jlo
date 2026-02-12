use crate::harness::TestContext;
use std::fs;

#[test]
fn automerge_workflow_delegates_policy_to_jlo_command() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let automerge = fs::read_to_string(root.join(".github/workflows/jules-automerge.yml")).unwrap();

    assert!(automerge.contains("jlo workflow gh pr enable-automerge"));
    assert!(automerge.contains("ATTEMPTS=12"));
    assert!(automerge.contains("enablePullRequestAutoMerge"));
    assert!(automerge.contains("mergePullRequest"));
    assert!(!automerge.contains("\n    concurrency:\n"));

    assert!(!automerge.contains("allowed_prefixes"));
    assert!(!automerge.contains("git ls-tree"));
    assert!(!automerge.contains("in_scope"));
}
