use crate::harness::TestContext;
use crate::harness::jlo_config;
use std::fs;

#[test]
fn workflow_generate_writes_expected_files_to_custom_output_dir() {
    let ctx = TestContext::new();

    jlo_config::write_jlo_config(ctx.work_dir(), &[jlo_config::DEFAULT_TEST_CRON], 30);

    let output_dir = ctx.work_dir().join(".tmp/workflow-scaffold-generate/remote");
    ctx.cli()
        .args(["workflow", "generate", "remote", "--output-dir"])
        .arg(&output_dir)
        .assert()
        .success();

    assert!(
        output_dir.join(".github/workflows/jules-scheduled-workflows.yml").exists(),
        "Generated workflow file should exist"
    );
    assert!(output_dir.join(".github/workflows/jules-run-only-innovators.yml").exists());
    assert!(output_dir.join(".github/workflows/jules-implementer-pr.yml").exists());
    assert!(output_dir.join(".github/workflows/jules-automerge.yml").exists());
    assert!(output_dir.join(".github/workflows/jules-integrator.yml").exists());
}

#[test]
fn workflow_generate_uses_default_output_dir() {
    let ctx = TestContext::new();

    jlo_config::write_jlo_config(ctx.work_dir(), &[jlo_config::DEFAULT_TEST_CRON], 30);

    ctx.cli().args(["workflow", "generate", "remote"]).assert().success();

    assert!(
        ctx.work_dir().join(".github/workflows/jules-scheduled-workflows.yml").exists(),
        "Default generate output should exist in .github/"
    );
}

#[test]
fn workflow_generate_overwrites_by_default() {
    let ctx = TestContext::new();

    jlo_config::write_jlo_config(ctx.work_dir(), &[jlo_config::DEFAULT_TEST_CRON], 30);

    let output_dir = ctx.work_dir().join(".tmp/workflow-scaffold-generate/overwrite");
    let stale_workflow_path = output_dir.join(".github/workflows/jules-scheduled-workflows.yml");
    fs::create_dir_all(stale_workflow_path.parent().unwrap()).unwrap();
    fs::write(&stale_workflow_path, "stale workflow").unwrap();

    ctx.cli()
        .args(["workflow", "generate", "remote", "--output-dir"])
        .arg(&output_dir)
        .assert()
        .success();

    let updated_workflow =
        fs::read_to_string(&stale_workflow_path).expect("read generated workflow");
    assert!(
        updated_workflow.contains("Jules Scheduled Workflows"),
        "Generated workflow should replace stale content"
    );

    assert!(
        output_dir.join(".github/workflows/jules-scheduled-workflows.yml").exists(),
        "Generated workflow file should exist after overwrite"
    );
}
