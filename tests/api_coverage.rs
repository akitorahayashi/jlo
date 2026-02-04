use jlo::{init, init_workflows, doctor, DoctorOptions, update, template, setup_list, WorkflowRunnerMode};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

struct CwdGuard {
    original: std::path::PathBuf,
}

impl CwdGuard {
    fn new(path: &std::path::Path) -> Self {
        let original = env::current_dir().expect("failed to get current dir");
        env::set_current_dir(path).expect("failed to set current dir");
        Self { original }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.original);
    }
}

#[test]
#[serial]
fn test_api_coverage_full_flow() {
    let temp = TempDir::new().unwrap();
    let _guard = CwdGuard::new(temp.path());

    // 1. Init
    // Initialize git repo because jlo init requires it
    std::process::Command::new("git")
        .arg("init")
        .status()
        .expect("failed to init git");

    // Also need to be on a branch named 'jules' or similar?
    // Memory says: "The `jlo init` command implementation ... strictly enforces that the current git branch is named `jules` before proceeding."
    // Let's create and checkout branch 'jules'.
    std::process::Command::new("git")
        .args(["checkout", "-b", "jules"])
        .status()
        .or_else(|_| {
            // If checkout -b fails (maybe initial commit needed?), try creating it differently.
            // Actually on empty repo, checkout -b works if it's the first branch? No, HEAD points to nothing.
            // Need an initial commit first.
            std::process::Command::new("git")
                .args(["commit", "--allow-empty", "-m", "initial", "--no-gpg-sign"])
                .status()
                .and_then(|_| {
                    std::process::Command::new("git")
                        .args(["checkout", "-b", "jules"])
                        .status()
                })
        })
        .expect("failed to setup jules branch");

    init().expect("init failed");
    assert!(temp.path().join(".jules").exists());

    // 2. Doctor (fresh init should pass)
    let doctor_outcome = doctor(DoctorOptions {
        fix: false,
        strict: false,
        workstream: None,
    }).expect("doctor failed");
    assert_eq!(doctor_outcome.exit_code, 0);

    // 3. Update (dry run)
    let update_result = update(true, false).expect("update failed");
    assert!(update_result.dry_run);

    // 4. Template (create role in generic workstream)
    // "generic" workstream is created by init
    // Note: Roles are created in global layer directory, not workstream directory.
    let _ = template(Some("observers"), Some("test-observer"), Some("generic"))
        .expect("template role failed");

    // Check global role location: .jules/roles/<layer>/roles/<role>
    let role_path = temp.path().join(".jules/roles/observers/roles/test-observer");
    assert!(role_path.exists(), "Role not found at {:?}", role_path);

    // 5. Init workflows
    init_workflows(WorkflowRunnerMode::Remote, false).expect("init workflows failed");
    // Check if one of the workflows exists
    // Note: The actual path depends on what init_workflows does.
    // It usually creates .github/workflows
    assert!(temp.path().join(".github/workflows").exists());
}

#[test]
#[serial]
fn test_api_coverage_setup() {
    let components = setup_list().expect("setup_list failed");
    assert!(!components.is_empty());
}
