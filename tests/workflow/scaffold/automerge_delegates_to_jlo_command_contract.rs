use crate::harness::TestContext;
use std::fs;

#[test]
fn automerge_path_delegates_policy_to_jlo_process_command() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-automerge.yml")).unwrap();

    assert!(workflow.contains("validate-and-automerge:"));
    assert!(workflow.contains("jlo workflow gh pr process"));
    assert!(workflow.contains("--mode automerge"));
    assert!(workflow.contains("--retry-attempts 12"));
    assert!(workflow.contains("--retry-delay-seconds 10"));
    assert!(workflow.contains("--fail-on-error"));

    assert!(!workflow.contains("enablePullRequestAutoMerge|mergePullRequest"));
    assert!(!workflow.contains("Auto-merge transient GitHub state detected"));

    assert!(
        !fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml"))
            .unwrap()
            .contains("validate-and-automerge:"),
        "scheduled workflow should not inline automerge job"
    );
}
