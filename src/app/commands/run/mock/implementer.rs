use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::mock::identity::generate_mock_id;
use crate::domain::{AppError, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock implementers.
pub fn execute_mock_implementers<G, H, W>(
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
        AppError::MissingArgument("Issue path is required for implementers".to_string())
    })?;

    // Parse issue to get label and id
    let issue_path_str = issue_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid issue path".to_string()))?;

    let issue_content = workspace.read_file(issue_path_str)?;
    let (label, issue_id) = parse_issue_for_branch(&issue_content, issue_path)?;

    // Implementer branch format: jules-implementer-<label>-<id>-<short_description>
    let branch_name = format!(
        "jules-implementer-{}-{}-mock-{}",
        label,
        &issue_id[..6.min(issue_id.len())],
        config.scope
    );

    println!("Mock implementers: creating branch {}", branch_name);

    // Fetch and checkout from default branch (not jules)
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.default_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create minimal mock file to have a commit
    let mock_file_path = format!(".mock-{}", config.scope);
    let mock_content = format!(
        "# Mock implementation marker\n# Scope: {}\n# Issue: {}\n# Created: {}\n",
        config.scope,
        issue_id,
        Utc::now().to_rfc3339()
    );

    workspace.write_file(&mock_file_path, &mock_content)?;

    // Commit and push
    let mock_path = Path::new(&mock_file_path);
    let files: Vec<&Path> = vec![mock_path];
    git.commit_files(&format!("[mock-{}] implementer: mock implementation", config.scope), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR targeting default branch (NOT jules)
    let pr = github.create_pull_request(
        &branch_name,
        &config.default_branch,
        &format!("[mock-{}] Implementation: {}", config.scope, label),
        &format!(
            "Mock implementer run for workflow validation.\n\nScope: `{}`\nIssue: `{}`\nLabel: `{}`\n\n⚠️ This PR targets `{}` (not `jules`) - requires human review.",
            config.scope,
            issue_id,
            label,
            config.default_branch
        ),
    )?;

    // NOTE: Implementer PRs do NOT get auto-merge enabled
    println!("Mock implementers: created PR #{} ({}) - awaiting label", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_scope: config.scope.clone(),
    })
}

/// Parse issue content to extract label and ID for branch naming.
fn parse_issue_for_branch(content: &str, path: &Path) -> Result<(String, String), AppError> {
    // Try to extract from YAML
    let mut label = None;
    let mut id = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("label:") {
            label = Some(
                line.trim_start_matches("label:")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            );
        } else if line.starts_with("id:") {
            id = Some(
                line.trim_start_matches("id:")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            );
        }
    }

    // Fallback: try to extract label from path (issues/<label>/...)
    if label.is_none()
        && let Some(parent) = path.parent()
        && let Some(name) = parent.file_name()
    {
        label = Some(name.to_string_lossy().to_string());
    }

    // Fallback: generate ID if not found
    if id.is_none() {
        id = Some(generate_mock_id());
    }

    Ok((label.unwrap_or_else(|| "bugs".to_string()), id.unwrap_or_else(generate_mock_id)))
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
        let path = Path::new(".jules/workstreams/generic/exchange/issues/bugs/test.yml");
        let (label, id) = parse_issue_for_branch(content, path).unwrap();
        assert_eq!(label, "bugs");
        assert_eq!(id, "abc123");
    }
}
