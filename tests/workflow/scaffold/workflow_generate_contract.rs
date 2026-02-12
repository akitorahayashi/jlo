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
        output_dir.join(".github/workflows/jules-workflows.yml").exists(),
        "Generated workflow file should exist"
    );
}

#[test]
fn workflow_generate_uses_default_output_dir() {
    let ctx = TestContext::new();

    jlo_config::write_jlo_config(ctx.work_dir(), &[jlo_config::DEFAULT_TEST_CRON], 30);

    ctx.cli().args(["workflow", "generate", "remote"]).assert().success();

    assert!(
        ctx.work_dir().join(".github/workflows/jules-workflows.yml").exists(),
        "Default generate output should exist in .github/"
    );
}

#[test]
fn workflow_generate_overwrites_by_default() {
    let ctx = TestContext::new();

    jlo_config::write_jlo_config(ctx.work_dir(), &[jlo_config::DEFAULT_TEST_CRON], 30);

    let output_dir = ctx.work_dir().join(".tmp/workflow-scaffold-generate/overwrite");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(output_dir.join("old.txt"), "old content").unwrap();

    ctx.cli()
        .args(["workflow", "generate", "remote", "--output-dir"])
        .arg(&output_dir)
        .assert()
        .success();

    assert!(
        output_dir.join(".github/workflows/jules-workflows.yml").exists(),
        "Generated workflow file should exist after overwrite"
    );
}
