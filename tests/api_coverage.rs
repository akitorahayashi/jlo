use jlo::{
    DoctorOptions, WorkflowRunnerMode, doctor_at, init_at, init_workflows_at, setup_list,
    template_at, update_at,
};
use tempfile::TempDir;

#[test]
fn test_api_coverage_full_flow() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // 1. Init
    // Initialize git repo because jlo init requires it
    std::process::Command::new("git")
        .current_dir(root)
        .arg("init")
        .status()
        .expect("failed to init git");

    // Also need to be on a branch named 'jules' or similar?
    // Memory says: "The `jlo init` command implementation ... strictly enforces that the current git branch is named `jules` before proceeding."
    // Let's create and checkout branch 'jules'.
    // Need an initial commit first to create a branch.
    std::process::Command::new("git")
        .current_dir(root)
        .args(["commit", "--allow-empty", "-m", "initial", "--no-gpg-sign"])
        .status()
        .expect("failed to create initial commit");

    std::process::Command::new("git")
        .current_dir(root)
        .args(["checkout", "-b", "jules"])
        .status()
        .expect("failed to setup jules branch");

    init_at(root.to_path_buf()).expect("init failed");
    assert!(root.join(".jules").exists());

    // 2. Doctor (fresh init should pass)
    let doctor_outcome = doctor_at(
        root.to_path_buf(),
        DoctorOptions { fix: false, strict: false, workstream: None },
    )
    .expect("doctor failed");
    assert_eq!(doctor_outcome.exit_code, 0);

    // 3. Update (prompt preview)
    let update_result = update_at(root.to_path_buf(), true, false).expect("update failed");
    assert!(update_result.prompt_preview);

    // 4. Template (create role in generic workstream)
    // "generic" workstream is created by init
    // Note: Roles are created in global layer directory, not workstream directory.
    let _ =
        template_at(Some("observers"), Some("test-observer"), Some("generic"), root.to_path_buf())
            .expect("template role failed");

    // Check global role location: .jules/roles/<layer>/roles/<role>
    let role_path = root.join(".jules/roles/observers/roles/test-observer");
    assert!(role_path.exists(), "Role not found at {:?}", role_path);

    // 5. Init workflows
    init_workflows_at(root.to_path_buf(), WorkflowRunnerMode::Remote)
        .expect("init workflows failed");
    // Check if one of the workflows exists
    // Note: The actual path depends on what init_workflows does.
    // It usually creates .github/workflows
    assert!(root.join(".github/workflows").exists());
}

#[test]
fn test_api_coverage_setup() {
    let components = setup_list().expect("setup_list failed");
    assert!(!components.is_empty());
}
