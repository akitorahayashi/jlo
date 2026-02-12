use crate::harness::TestContext;
use crate::harness::jlo_config;
use jlo::{WorkflowRunnerMode, init_workflows_at};
use std::fs;

#[test]
fn init_workflows_preserves_unrelated_files() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    jlo_config::write_jlo_config(root, &[jlo_config::DEFAULT_TEST_CRON], 30);

    let scaffold_workflow = root.join(".github/workflows/jules-scheduled-workflows.yml");
    fs::create_dir_all(scaffold_workflow.parent().unwrap()).unwrap();
    fs::write(&scaffold_workflow, "old workflow").unwrap();

    let unrelated_workflow = root.join(".github/workflows/unrelated.yml");
    fs::write(&unrelated_workflow, "keep me").unwrap();

    let scaffold_action = root.join(".github/actions/install-jlo/action.yml");
    fs::create_dir_all(scaffold_action.parent().unwrap()).unwrap();
    fs::write(&scaffold_action, "old action").unwrap();

    let unrelated_action = root.join(".github/actions/custom/action.yml");
    fs::create_dir_all(unrelated_action.parent().unwrap()).unwrap();
    fs::write(&unrelated_action, "custom action").unwrap();

    // Use API directly — testing workflow scaffold re-install over existing files.
    init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote()).unwrap();

    let updated_workflow = fs::read_to_string(&scaffold_workflow).unwrap();
    assert!(updated_workflow.contains("Jules Scheduled Workflows"));

    let updated_action = fs::read_to_string(&scaffold_action).unwrap();
    assert!(updated_action.contains("Install jlo"));

    let unrelated_content = fs::read_to_string(&unrelated_workflow).unwrap();
    assert_eq!(unrelated_content, "keep me");

    let unrelated_action_content = fs::read_to_string(&unrelated_action).unwrap();
    assert_eq!(unrelated_action_content, "custom action");
}

#[test]
fn init_workflows_removes_stale_jules_workflows() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    jlo_config::write_jlo_config(root, &[jlo_config::DEFAULT_TEST_CRON], 30);

    let stale_paths = [
        root.join(".github/workflows/jules-workflows.yml"),
        root.join(".github/workflows/jules-sync.yml"),
        root.join(".github/workflows/jules-automerge.yml"),
        root.join(".github/workflows/jules-implementer-pr.yml"),
        root.join(".github/workflows/jules-implementer-label.yml"),
        root.join(".github/workflows/jules-pr-summary-request.yml"),
        root.join(".github/workflows/jules-mock-cleanup.yml"),
    ];

    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    for stale in &stale_paths {
        fs::write(stale, "legacy jlo-managed workflow").unwrap();
    }

    init_workflows_at(root.to_path_buf(), &WorkflowRunnerMode::remote()).unwrap();

    for stale in &stale_paths {
        assert!(
            !stale.exists(),
            "stale jlo-managed workflow should be removed: {}",
            stale.display()
        );
    }

    assert!(
        root.join(".github/workflows/jules-scheduled-workflows.yml").exists(),
        "current primary workflow should be installed"
    );
}
