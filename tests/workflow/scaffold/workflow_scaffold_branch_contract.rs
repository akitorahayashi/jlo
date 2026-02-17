use crate::harness::TestContext;
use std::fs;

fn rendered_env_value(workflow: &str, key: &str) -> String {
    let prefix = format!("{}: '", key);
    let line = workflow
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))
        .unwrap_or_else(|| panic!("{} should be rendered in workflow env", key));
    let after = line
        .trim_start()
        .strip_prefix(&prefix)
        .expect("env value should start with expected prefix");
    after.strip_suffix('\'').expect("env value should end with quote").to_string()
}

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

    let worker_branch = rendered_env_value(&primary, "JULES_WORKER_BRANCH");

    // Implementer job should check out worker branch context, but dispatch API on target branch.
    let implementer_section =
        primary.split("run-implementer:").nth(1).expect("run-implementer job should exist");
    let implementer_checkout = implementer_section
        .split("actions/checkout@")
        .nth(1)
        .expect("implementer should have checkout step");
    assert!(
        implementer_checkout.contains(&format!("ref: '{}'", worker_branch)),
        "Implementer job should check out worker branch context"
    );
    assert!(
        implementer_section.contains("--branch \"${JLO_TARGET_BRANCH}\""),
        "Implementer job should dispatch Jules API on target branch"
    );

    // Integrator workflow should check out target branch
    let integrator =
        fs::read_to_string(root.join(".github/workflows/jules-integrator.yml")).unwrap();
    let integrator_checkout =
        integrator.split("actions/checkout@").nth(1).expect("integrator should have checkout step");
    let integrator_target = rendered_env_value(&integrator, "JLO_TARGET_BRANCH");
    assert!(
        integrator_checkout.contains(&format!("ref: '{}'", integrator_target)),
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
