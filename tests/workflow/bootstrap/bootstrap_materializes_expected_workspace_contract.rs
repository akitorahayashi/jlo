use crate::harness::TestContext;

#[test]
fn bootstrap_materializes_expected_runtime_workspace() {
    let ctx = TestContext::new();

    ctx.init_remote_and_bootstrap();

    ctx.assert_jlo_exists();
    ctx.assert_jules_exists();
    assert!(ctx.read_version().is_some());

    ctx.assert_schema_structure_exists();
    ctx.assert_default_scheduled_roles_exist();
    ctx.assert_exchange_structure_exists();
    ctx.assert_events_structure_exists();
    ctx.assert_requirements_directory_exists();
    ctx.assert_contracts_available();
    ctx.assert_narrator_exists();

    // Verify specific files.
    let root_files = ["JULES.md", "README.md", ".jlo-version", "github-labels.json"];
    for file in root_files {
        assert!(ctx.jules_path().join(file).exists(), "{} should exist in .jules/ (runtime)", file);
    }
    assert!(
        !ctx.jules_path().join(".jlo-managed.yml").exists(),
        ".jlo-managed manifest should not be materialized"
    );

    assert!(ctx.jlo_path().join("config.toml").exists());
}
