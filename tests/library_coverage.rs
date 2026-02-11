use jlo::{
    DoctorOptions, WorkflowRunnerMode, create_role_at, doctor_at, init_at, update_at,
    workflow_bootstrap_at,
};
use tempfile::TempDir;

#[test]
fn test_library_lifecycle_coverage() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    // 1. Initialize git repo (required for init)
    let git_init_status = std::process::Command::new("git")
        .current_dir(&root)
        .arg("init")
        .status()
        .expect("failed to init git");
    assert!(git_init_status.success());

    // 2. Init
    init_at(root.clone(), &WorkflowRunnerMode::remote()).expect("init failed");
    assert!(root.join(".jlo").exists());

    // 3. Bootstrap
    workflow_bootstrap_at(root.clone()).expect("bootstrap failed");
    assert!(root.join(".jules").exists());

    // 4. Doctor
    let doctor_outcome =
        doctor_at(root.clone(), DoctorOptions { strict: false }).expect("doctor failed");
    assert_eq!(doctor_outcome.exit_code, 0);

    // 5. Create Role
    let outcome =
        create_role_at("observers", "lib-observer", root.clone()).expect("create role failed");
    assert_eq!(outcome.entity_type(), "role");
    assert!(root.join(".jlo/roles/observers/lib-observer/role.yml").exists());

    // 6. Update
    let update_result = update_at(root.clone(), true).expect("update failed");
    assert!(update_result.prompt_preview);
}
