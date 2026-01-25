mod common;

use common::TestContext;
use rs_cli_tmpl::{add, delete, list};
use serial_test::serial;

#[test]
#[serial]
fn add_persists_item_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_dir(ctx.work_dir(), || {
        add("sample", "hello world").expect("library add should succeed");
    });

    ctx.assert_saved_item_contains("sample", "hello world");
}

#[test]
#[serial]
fn list_returns_items_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_dir(ctx.work_dir(), || {
        add("first", "one").expect("add should succeed");
        add("second", "two").expect("add should succeed");
        let mut items = list().expect("list should succeed");
        items.sort();
        assert_eq!(items, vec!["first".to_string(), "second".to_string()]);
    });
}

#[test]
#[serial]
fn delete_removes_item_via_library_api() {
    let ctx = TestContext::new();

    ctx.with_dir(ctx.work_dir(), || {
        add("temp", "value").expect("add should succeed");
    });

    assert!(ctx.saved_item_path("temp").exists(), "Item should exist before delete");

    ctx.with_dir(ctx.work_dir(), || {
        delete("temp").expect("delete should succeed");
    });

    assert!(!ctx.saved_item_path("temp").exists(), "Item should be removed after delete");
}
