mod common;

use common::TestContext;

#[test]
fn init_fails_if_exists() {
    let ctx = TestContext::new();

    jlo::init_at(ctx.work_dir().to_path_buf(), jlo::WorkflowRunnerMode::Remote)
        .expect("first init should succeed");
    let err = jlo::init_at(ctx.work_dir().to_path_buf(), jlo::WorkflowRunnerMode::Remote)
        .expect_err("second init should fail");
    assert!(matches!(err, jlo::AppError::WorkspaceExists));
}

#[test]
fn create_role_without_workspace_fails() {
    let ctx = TestContext::new();

    let err = jlo::create_role_at("observers", "test", ctx.work_dir().to_path_buf())
        .expect_err("create should fail without workspace");
    assert!(matches!(err, jlo::AppError::WorkspaceNotFound));
}
