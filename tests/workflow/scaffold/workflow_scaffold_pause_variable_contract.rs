use crate::harness::TestContext;
use std::fs;

#[test]
fn installed_workflow_scaffold_uses_jlo_paused_variable() {
    let ctx = TestContext::new();

    ctx.init_remote();

    let root = ctx.work_dir();
    let workflow =
        fs::read_to_string(root.join(".github/workflows/jules-scheduled-workflows.yml")).unwrap();

    assert!(workflow.contains("vars.JLO_PAUSED"));
    assert!(!workflow.contains("vars.JULES_PAUSED"));
    assert!(
        workflow
            .contains("(vars.JLO_PAUSED || 'false') != 'true' || github.event_name != 'schedule'")
    );
}
