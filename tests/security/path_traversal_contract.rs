use crate::harness::TestContext;
use jlo::adapters::git::GitCommandAdapter;
use jlo::ports::Git;

#[test]
fn harness_paths_are_contained() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();

    assert!(ctx.jlo_path().starts_with(root), "jlo_path should be inside work_dir");
    assert!(ctx.jules_path().starts_with(root), "jules_path should be inside work_dir");
    assert!(ctx.layers_path().starts_with(root), "layers_path should be inside work_dir");
    assert!(ctx.exchange_path().starts_with(root), "exchange_path should be inside work_dir");
}

#[test]
fn cannot_commit_files_outside_repo() {
    let ctx = TestContext::new();
    let git = GitCommandAdapter::new(ctx.work_dir().to_path_buf());

    // Create a file outside the repo (but within harness root)
    let outside_file = ctx.home().join("secret.txt");
    std::fs::write(&outside_file, "secret").expect("Failed to create outside file");

    // Attempt to commit
    let result = git.commit_files("steal secrets", &[&outside_file]);

    // Expect failure as git should reject files outside the repo
    assert!(result.is_err(), "Git should reject committing files outside the repo");
}
