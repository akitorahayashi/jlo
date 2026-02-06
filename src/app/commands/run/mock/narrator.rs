use std::path::Path;

use chrono::Utc;

use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock narrator.
pub fn execute_mock_narrator<G, H, W>(
    jules_path: &Path,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore,
{
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Narrators, &timestamp)?;

    println!("Mock narrator: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create mock changes file
    let changes_dir = jules_path.join("changes");
    let changes_file = changes_dir.join("latest.yml");

    // Use pre-defined mock template
    let mock_change_template = super::MOCK_ASSETS
        .get_file("narrator_change.yml")
        .expect("Mock asset missing: narrator_change.yml")
        .contents_utf8()
        .expect("Invalid UTF-8 in narrator_change.yml");
    let mock_id = format!("mock{:04x}", (Utc::now().timestamp() % 0x10000) as u16);
    let now = Utc::now();

    // Replace placeholders in template
    let changes_content = mock_change_template
        .replace("mock01", &mock_id[..6])
        .replace("2026-02-05T00:00:00Z", &now.to_rfc3339())
        .replace(
            "Mock narrator run for workflow validation",
            &format!("Mock narrator run for workflow validation\n# Mock tag: {}", config.mock_tag),
        );

    workspace.write_file(
        changes_file.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &changes_content,
    )?;

    // Commit and push
    let files: Vec<&Path> = vec![changes_file.as_path()];
    git.commit_files(&format!("[mock-{}] narrator: mock changes", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Narrator changes", config.mock_tag),
        &format!("Mock narrator run for workflow validation.\n\nMock tag: `{}`", config.mock_tag),
    )?;

    println!("Mock narrator: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}
