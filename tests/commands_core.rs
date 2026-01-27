mod common;

use common::TestContext;
use serial_test::serial;
use std::io;

#[test]
#[serial]
fn init_fails_if_exists() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jlo::init().expect("first init should succeed");
        let err = jlo::init().expect_err("second init should fail");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    });
}

#[test]
#[serial]
fn template_without_workspace_fails() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        let err = jlo::template(Some("observers"), Some("test")).expect_err("template should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    });
}

#[test]
#[serial]
fn assign_without_workspace_fails() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        let err = jlo::assign("taxonomy", &[]).expect_err("assign should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    });
}
