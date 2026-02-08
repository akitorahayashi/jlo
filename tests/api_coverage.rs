use jlo::{
    DoctorOptions, WorkflowRunnerMode, create_role_at, doctor_at, init_at, setup_list, update_at,
    workflow_bootstrap_at,
};
use tempfile::TempDir;

#[test]
fn test_api_coverage_full_flow() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    // Initialize git repo
    let git_init_status = std::process::Command::new("git")
        .current_dir(&root)
        .arg("init")
        .status()
        .expect("failed to init git");
    assert!(git_init_status.success(), "git init should succeed");

    // Init requires a non-jules branch (control branch)
    // Default branch after git init is typically 'main' or 'master'; that's fine.

    init_at(root.clone(), WorkflowRunnerMode::Remote).expect("init failed");
    assert!(root.join(".jlo").exists(), ".jlo/ control plane should exist");
    workflow_bootstrap_at(root.clone()).expect("bootstrap failed");
    assert!(root.join(".jules").exists(), ".jules/ runtime workspace should exist");
    assert!(root.join(".github/workflows").exists(), "workflow kit should be installed");

    // 2. Doctor (fresh init should pass)
    let doctor_outcome = doctor_at(root.clone(), DoctorOptions { strict: false, workstream: None })
        .expect("doctor failed");
    assert_eq!(doctor_outcome.exit_code, 0);

    // 3. Update (prompt preview)
    let update_result = update_at(root.clone(), true).expect("update failed");
    assert!(update_result.prompt_preview);

    // 4. Create role under .jlo/ control plane
    let outcome =
        create_role_at("observers", "test-observer", root.clone()).expect("create role failed");
    assert_eq!(outcome.entity_type(), "role");

    // Role should exist in .jlo/ control plane
    let jlo_role_path = root.join(".jlo/roles/observers/roles/test-observer");
    assert!(jlo_role_path.exists(), "Role not found in .jlo/ at {:?}", jlo_role_path);
}

#[test]
fn test_api_coverage_setup() {
    let components = setup_list().expect("setup_list failed");
    assert!(!components.is_empty());
}
