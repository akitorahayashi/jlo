use crate::harness::TestContext;
use std::fs;

#[test]
fn automerge_path_delegates_policy_to_jlo_process_command() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow = fs::read_to_string(root.join(".github/workflows/jules-automerge.yml")).unwrap();

    assert!(workflow.contains("validate-and-automerge:"));
    assert!(workflow.contains("push:"));
    assert!(workflow.contains("github.event.deleted == false"));
    assert!(workflow.contains("Discover open PR for pushed branch"));
    assert!(workflow.contains("--repo \"${GITHUB_REPOSITORY}\""));
    assert!(workflow.contains("--head \"${GITHUB_REF_NAME}\""));
    assert!(workflow.contains("--base \"${JULES_WORKER_BRANCH}\""));
    assert!(workflow.contains("for attempt in $(seq 1 12); do"));
    assert!(workflow.contains("after 12 attempts"));
    assert!(workflow.contains("jlo workflow gh pr process automerge"));
    assert!(workflow.contains("steps.discover.outputs.pr_number"));
    assert!(workflow.contains("--retry-attempts 12"));
    assert!(workflow.contains("--retry-delay-seconds 10"));
    assert!(workflow.contains("--fail-on-error"));
    assert!(workflow.contains("format('jules-automerge-branch-{0}', github.ref_name)"));

    assert!(!workflow.contains("pull_request:"));
    assert!(!workflow.contains("github.event.pull_request.number"));
    assert!(!workflow.contains("--mode automerge"));

    assert!(!workflow.contains("enablePullRequestAutoMerge|mergePullRequest"));
    assert!(!workflow.contains("Auto-merge transient GitHub state detected"));

    assert!(
        !fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml"))
            .unwrap()
            .contains("validate-and-automerge:"),
        "scheduled workflow should not inline automerge job"
    );
}
