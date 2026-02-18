use crate::harness::git_repository;
use jlo::{
    DoctorOptions, WorkflowRunnerMode, doctor_at, init_at, role_create_at, update_at,
    workflow_bootstrap_managed_files_at, workflow_bootstrap_workstations_at,
};
use tempfile::TempDir;

#[test]
fn public_api_lifecycle_happy_path_contract() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    // init_at requires a git repo.
    let output = std::process::Command::new("git")
        .current_dir(&root)
        .arg("init")
        .output()
        .expect("failed to init git");
    assert!(output.status.success());

    git_repository::configure_user(&root);

    init_at(root.clone(), &WorkflowRunnerMode::remote()).expect("init failed");
    assert!(root.join(".jlo").exists());

    workflow_bootstrap_managed_files_at(root.clone()).expect("managed-files bootstrap failed");
    workflow_bootstrap_workstations_at(root.clone()).expect("workstations bootstrap failed");
    assert!(root.join(".jules").exists());

    let doctor_outcome =
        doctor_at(root.clone(), DoctorOptions { strict: false }).expect("doctor failed");
    assert_eq!(doctor_outcome.exit_code, 0);

    let outcome =
        role_create_at("observers", "lib-observer", root.clone()).expect("create role failed");
    assert_eq!(outcome.entity_type(), "role");
    assert!(root.join(".jlo/roles/observers/lib-observer/role.yml").exists());

    let update_result = update_at(root.clone(), true).expect("update failed");
    assert!(update_result.prompt_preview);
}
