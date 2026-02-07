use jlo::{
    DoctorOptions, WorkflowRunnerMode, doctor_at, init_at, setup_list, template_at, update_at,
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
    assert!(root.join(".jules").exists(), ".jules/ runtime workspace should exist");
    assert!(root.join(".github/workflows").exists(), "workflow kit should be installed");

    // 2. Doctor (fresh init should pass)
    let doctor_outcome =
        doctor_at(root.clone(), DoctorOptions { fix: false, strict: false, workstream: None })
            .expect("doctor failed");
    assert_eq!(doctor_outcome.exit_code, 0);

    // 3. Update (prompt preview)
    let update_result = update_at(root.clone(), true).expect("update failed");
    assert!(update_result.prompt_preview);

    // 4. Template (create role in generic workstream)
    let _ = template_at(Some("observers"), Some("test-observer"), Some("generic"), root.clone())
        .expect("template role failed");

    // Check global role location: .jules/roles/<layer>/roles/<role>
    let role_path = root.join(".jules/roles/observers/roles/test-observer");
    assert!(role_path.exists(), "Role not found at {:?}", role_path);
}

#[test]
fn test_api_coverage_setup() {
    let components = setup_list().expect("setup_list failed");
    assert!(!components.is_empty());
}
