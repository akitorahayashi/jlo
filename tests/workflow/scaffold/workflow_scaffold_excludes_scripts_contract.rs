use crate::harness::TestContext;
use std::fs;

#[test]
fn installed_workflow_scaffold_has_no_script_references() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow_dir = root.join(".github/workflows");

    let workflow = fs::read_to_string(workflow_dir.join("jules-workflows.yml")).unwrap();
    assert!(!workflow.contains(".github/scripts/"));

    for entry in fs::read_dir(&workflow_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|ext| ext == "yml") {
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains(".github/scripts/"),
                "Workflow {} should not reference .github/scripts/",
                entry.path().display()
            );
        }
    }

    for action_dir in ["install-jlo", "configure-git"] {
        let action_path = root.join(format!(".github/actions/{}/action.yml", action_dir));
        if action_path.exists() {
            let content = fs::read_to_string(&action_path).unwrap();
            assert!(!content.contains(".github/scripts/"));
        }
    }
}
