mod common;

use common::TestContext;
use serial_test::serial;

#[test]
#[serial]
fn init_creates_workspace_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init().expect("init should succeed");
    });

    ctx.assert_jules_exists();
}

#[test]
#[serial]
fn update_refreshes_files_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_work_dir(|| {
        jo::init().expect("init should succeed");
        jo::update().expect("update should succeed");
    });
}
