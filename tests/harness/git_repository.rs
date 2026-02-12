use std::path::Path;

pub(crate) fn configure_user(repo_dir: &Path) {
    let output = std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_dir)
        .output()
        .expect("git config email failed");
    assert!(
        output.status.success(),
        "git config user.email failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_dir)
        .output()
        .expect("git config name failed");
    assert!(
        output.status.success(),
        "git config user.name failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(crate) fn init_bare_repo(path: &Path) {
    let output = std::process::Command::new("git")
        .args(["init", "--bare"])
        .current_dir(path)
        .output()
        .expect("git init bare failed");
    assert!(output.status.success(), "git init bare failed: {}", String::from_utf8_lossy(&output.stderr));
}

pub(crate) fn add_origin_remote(repo_dir: &Path, url: &str) {
    let output = std::process::Command::new("git")
        .args(["remote", "add", "origin", url])
        .current_dir(repo_dir)
        .output()
        .expect("git remote add failed");
    assert!(
        output.status.success(),
        "git remote add origin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(crate) fn commit_all(repo_dir: &Path, message: &str) {
    let output = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_dir)
        .output()
        .expect("git add failed");
    assert!(output.status.success(), "git add failed: {}", String::from_utf8_lossy(&output.stderr));

    let output = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_dir)
        .output()
        .expect("git commit failed");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
