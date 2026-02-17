use crate::harness::TestContext;
use std::fs;

#[test]
fn installed_workflow_scaffold_enforces_explicit_branch_contract() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let primary =
        fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml")).unwrap();
    assert!(primary.contains("JLO_TARGET_BRANCH"));
    assert!(primary.contains("JULES_WORKER_BRANCH"));
    assert!(primary.contains("bootstrap:"));
    assert!(!primary.contains("process-implementer-pr-metadata:"));
    assert!(!primary.contains("validate-and-automerge:"));
    assert!(root.join(".github/workflows/jules-implementer-pr.yml").exists());
    assert!(root.join(".github/workflows/jules-automerge.yml").exists());

    // Implementer job should check out target branch, not worker branch
    let implementer_section =
        primary.split("run-implementer:").nth(1).expect("run-implementer job should exist");
    let implementer_checkout = implementer_section
        .split("actions/checkout@")
        .nth(1)
        .expect("implementer should have checkout step");
    assert!(
        implementer_checkout.contains("ref: 'main'"),
        "Implementer job should check out target branch, not worker branch"
    );

    // Integrator workflow should check out target branch
    let integrator =
        fs::read_to_string(root.join(".github/workflows/jules-integrator.yml")).unwrap();
    let integrator_checkout =
        integrator.split("actions/checkout@").nth(1).expect("integrator should have checkout step");
    assert!(
        integrator_checkout.contains("ref: 'main'"),
        "Integrator workflow should check out target branch"
    );

    for entry in fs::read_dir(root.join(".github/workflows")).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|ext| ext == "yml") {
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains("github.event.repository.default_branch"),
                "Workflow {} should not reference github.event.repository.default_branch",
                entry.path().display()
            );
            assert!(
                !content.contains(".jlo-control"),
                "Workflow {} should not reference .jlo-control",
                entry.path().display()
            );
        }
    }

    for action_dir in ["install-jlo", "configure-git"] {
        let action_path = root.join(format!(".github/actions/{}/action.yml", action_dir));
        if action_path.exists() {
            let content = fs::read_to_string(&action_path).unwrap();
            assert!(!content.contains("github.event.repository.default_branch"));
            assert!(!content.contains(".jlo-control"));
        }
    }
}
