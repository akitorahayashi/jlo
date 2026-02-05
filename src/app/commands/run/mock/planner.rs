use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock planners.
pub fn execute_mock_planners<G, H, W>(
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
        AppError::MissingArgument("Issue path is required for planners".to_string())
    })?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Planners, &timestamp)?;

    println!("Mock planners: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Read and modify issue file
    let issue_path_str = issue_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid issue path".to_string()))?;

    let issue_content = workspace.read_file(issue_path_str)?;

    // Update issue: expand analysis and set requires_deep_analysis to false
    let updated_content = issue_content
        .replace("requires_deep_analysis: true", "requires_deep_analysis: false")
        + &format!(
            r#"
# Mock planner expansion
expanded_at: "{}"
expanded_by: mock-planner
analysis_details: |
  Mock deep analysis performed by jlo --mock for workflow validation.
  Mock tag: {}
  
  ## Impact Analysis
  - Mock impact area 1
  - Mock impact area 2
  
  ## Implementation Notes
  - No actual analysis performed (mock mode)
"#,
            Utc::now().to_rfc3339(),
            config.mock_tag
        );

    workspace.write_file(issue_path_str, &updated_content)?;

    // Commit and push
    let files: Vec<&Path> = vec![issue_path.as_path()];
    git.commit_files(&format!("[mock-{}] planner: analysis complete", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Planner analysis", config.mock_tag),
        &format!(
            "Mock planner run for workflow validation.\n\nMock tag: `{}`\nIssue: `{}`",
            config.mock_tag,
            issue_path.display()
        ),
    )?;

    println!("Mock planners: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}
