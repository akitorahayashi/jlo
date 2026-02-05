mod common;

use common::TestContext;

#[test]
fn init_fails_if_exists() {
    let ctx = TestContext::new();

    jlo::init_at(ctx.work_dir().to_path_buf()).expect("first init should succeed");
    let err = jlo::init_at(ctx.work_dir().to_path_buf()).expect_err("second init should fail");
    assert!(matches!(err, jlo::AppError::WorkspaceExists));
}

#[test]
fn template_without_workspace_fails() {
    let ctx = TestContext::new();

    let err = jlo::template_at(Some("observers"), Some("test"), None, ctx.work_dir().to_path_buf())
        .expect_err("template should fail");
    assert!(matches!(err, jlo::AppError::WorkspaceNotFound));
}
