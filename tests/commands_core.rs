mod common;

use common::TestContext;
use serial_test::serial;
use std::io;

#[test]
#[serial]
fn init_without_force_fails_if_exists() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init(false).expect("first init should succeed");
        let err = jo::init(false).expect_err("second init should fail");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    });
}

#[test]
#[serial]
fn role_with_invalid_id_fails() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init(false).expect("init should succeed");
        let err = jo::role("invalid/id").expect_err("role should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    });
}

#[test]
#[serial]
fn session_without_role_fails() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init(false).expect("init should succeed");
        let err = jo::session("nonexistent", None).expect_err("session should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    });
}

#[test]
#[serial]
fn update_without_workspace_fails() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        let err = jo::update(false).expect_err("update should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    });
}
