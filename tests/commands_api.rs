mod common;

use common::TestContext;
use serial_test::serial;

#[test]
#[serial]
fn init_creates_workspace_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init(false).expect("init should succeed");
    });

    ctx.assert_jules_exists();
}

#[test]
#[serial]
fn status_returns_info_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        // Status without workspace
        jo::status().expect("status should succeed even without workspace");

        // Initialize and check again
        jo::init(false).expect("init should succeed");
        jo::status().expect("status should succeed with workspace");
    });
}

#[test]
#[serial]
fn role_creates_directory_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init(false).expect("init should succeed");
        jo::role("value").expect("role should succeed");
    });

    ctx.assert_role_exists("value");
}

#[test]
#[serial]
fn session_creates_file_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init(false).expect("init should succeed");
        jo::role("value").expect("role should succeed");
        let path = jo::session("value", Some("test-session")).expect("session should succeed");
        assert!(path.exists());
    });
}

#[test]
#[serial]
fn update_refreshes_files_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init(false).expect("init should succeed");
        jo::update(false).expect("update should succeed");
    });
}
