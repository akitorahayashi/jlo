use crate::harness::TestContext;
use std::fs;

#[test]
fn bootstrap_does_not_delete_unmanaged_files_in_runtime_plane() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    let random_file = ctx.jules_path().join("random.txt");
    fs::write(&random_file, "I survive").expect("write random file");

    ctx.cli().args(["workflow", "bootstrap", "managed-files"]).assert().success();

    assert!(random_file.exists(), "Unmanaged file in .jules/ should survive bootstrap");
}
