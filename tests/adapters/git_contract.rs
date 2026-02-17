use crate::harness::TestContext;
use jlo::adapters::git::GitCommandAdapter;
use jlo::ports::Git;
use std::fs;

#[test]
fn git_contract_tests() {
    let ctx = TestContext::new();
    let root = ctx.work_dir();
    let git = GitCommandAdapter::new(root.to_path_buf());

    // 1. Initial state (TestContext inits a repo with 'main')
    let branch = git.get_current_branch().expect("get branch");
    assert_eq!(branch, "main");

    // 2. Commit files
    let file_path = root.join("test.txt");
    fs::write(&file_path, "content").unwrap();
    let sha1 = git.commit_files("commit 1", &[&file_path]).expect("commit");

    assert!(git.commit_exists(&sha1), "Commit sha1 should exist");
    assert_eq!(git.get_head_sha().unwrap(), sha1, "HEAD should match sha1");

    // 3. Changes
    fs::write(&file_path, "changed").unwrap();

    // has_changes compares commits, so we need another commit to compare.
    let sha2 = git.commit_files("commit 2", &[&file_path]).expect("commit 2");

    assert!(
        git.has_changes(&sha1, &sha2, &["test.txt"]).unwrap(),
        "Should have changes between sha1 and sha2"
    );
    assert!(
        !git.has_changes(&sha1, &sha1, &["test.txt"]).unwrap(),
        "Should not have changes between sha1 and sha1"
    );

    // 4. Ancestry
    let ancestor = git.get_nth_ancestor(&sha2, 1).expect("ancestor");
    assert_eq!(ancestor, sha1, "Ancestor of sha2 should be sha1");

    // 5. Checkout
    git.checkout_branch("feature", true).expect("checkout new branch");
    assert_eq!(git.get_current_branch().unwrap(), "feature");

    // 6. Delete branch
    git.checkout_branch("main", false).expect("checkout main");
    assert!(git.delete_branch("feature", false).unwrap(), "Should delete feature branch");
    // Verify it's gone
    let branches = git.run_command(&["branch"], None).unwrap();
    assert!(!branches.contains("feature"));

    assert!(
        !git.delete_branch("non-existent", false).unwrap(),
        "Should return false for non-existent branch"
    );

    // 7. Remote operations (Push/Fetch)
    // Create a bare repo as remote
    let remote_dir = ctx.home().join("remote.git");
    fs::create_dir_all(&remote_dir).unwrap();
    std::process::Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .current_dir(&remote_dir)
        .output()
        .expect("init remote");

    // Add remote
    git.run_command(&["remote", "add", "origin", remote_dir.to_str().unwrap()], None)
        .expect("add remote");

    // Push
    git.push_branch("main", false).expect("push main");

    // Fetch
    git.fetch("origin").expect("fetch origin");
}
