use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock implementer.
pub fn execute_mock_implementer<G, H, W>(
    _jules_path: &Path,
    options: &RunOptions,
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
    let issue_path = options.issue.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Issue path is required for implementer".to_string())
    })?;

    // Parse issue to get label and id
    let issue_path_str = issue_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid issue path".to_string()))?;

    let issue_content = workspace.read_file(issue_path_str)?;
    let (label, issue_id) = parse_issue_for_branch(&issue_content, issue_path)?;
    if !config.issue_labels.contains(&label) {
        return Err(AppError::Validation(format!(
            "Issue label '{}' is not defined in github-labels.json",
            label
        )));
    }

    // Implementer branch format: jules-implementer-<label>-<id>-<short_description>
    let prefix = config.branch_prefix(Layer::Implementer)?;
    let issue_id_short = issue_id.chars().take(6).collect::<String>();
    let branch_name = format!("{}{}-{}-{}", prefix, label, issue_id_short, config.mock_tag);

    println!("Mock implementer: creating branch {}", branch_name);

    // Fetch and checkout from default branch (not jules)
    git.fetch("origin")?;
    let base_branch = options.branch.as_deref().unwrap_or(&config.default_branch);
    git.checkout_branch(&format!("origin/{}", base_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create minimal mock file to have a commit
    let mock_file_path = format!(".mock-{}", config.mock_tag);
    let mock_content = format!(
        "# Mock implementation marker\n# Mock tag: {}\n# Issue: {}\n# Created: {}\n",
        config.mock_tag,
        issue_id,
        Utc::now().to_rfc3339()
    );

    workspace.write_file(&mock_file_path, &mock_content)?;

    // Commit and push
    let mock_path = Path::new(&mock_file_path);
    let files: Vec<&Path> = vec![mock_path];
    git.commit_files(&format!("[{}] implementer: mock implementation", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR targeting default branch (NOT jules)
    let pr = github.create_pull_request(
        &branch_name,
        base_branch,
        &format!("[{}] Implementation: {}", config.mock_tag, label),
        &format!(
            "Mock implementer run for workflow validation.\n\nMock tag: `{}`\nIssue: `{}`\nLabel: `{}`\n\n⚠️ This PR targets `{}` (not `jules`) - requires human review.",
            config.mock_tag,
            issue_id,
            label,
            base_branch
        ),
    )?;

    // NOTE: Implementer PRs do NOT get auto-merge enabled
    println!("Mock implementer: created PR #{} ({}) - awaiting label", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}

/// Parse issue content to extract label and ID for branch naming.
fn parse_issue_for_branch(content: &str, path: &Path) -> Result<(String, String), AppError> {
    let mut label = None;
    let mut id = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("label:") {
            let value =
                line.trim_start_matches("label:").trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                label = Some(value.to_string());
            }
        } else if line.starts_with("id:") {
            let value = line.trim_start_matches("id:").trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                id = Some(value.to_string());
            }
        }
    }

    let label = label.ok_or_else(|| {
        AppError::Validation(format!("Issue file missing label field: {}", path.display()))
    })?;
    let id = id.ok_or_else(|| {
        AppError::Validation(format!("Issue file missing id field: {}", path.display()))
    })?;

    if id.len() != 6 || !id.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit()) {
        return Err(AppError::Validation(format!(
            "Issue id must be 6 lowercase alphanumeric chars: {}",
            path.display()
        )));
    }

    Ok((label, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_issue_for_branch() {
        let content = r#"
id: "abc123"
label: "bugs"
title: "Test issue"
"#;
        let path = Path::new(".jules/exchange/requirements/test.yml");
        let (label, id) = parse_issue_for_branch(content, path).unwrap();
        assert_eq!(label, "bugs");
        assert_eq!(id, "abc123");
    }
}
