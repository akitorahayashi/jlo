use std::path::Path;

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock planner.
pub fn execute_mock_planner<G, H, W>(
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
    let requirement_path = options.requirement.as_ref().ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for planner".to_string())
    })?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Planner, &timestamp)?;

    println!("Mock planner: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Read and modify requirement file
    let requirement_path_str = requirement_path
        .to_str()
        .ok_or_else(|| AppError::Validation("Invalid requirement path".to_string()))?;

    let requirement_content = workspace.read_file(requirement_path_str)?;

    // Update requirement: expand analysis and set requires_deep_analysis to false
    let updated_content = requirement_content
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

    workspace.write_file(requirement_path_str, &updated_content)?;

    // Commit and push
    let files: Vec<&Path> = vec![requirement_path.as_path()];
    git.commit_files(&format!("[{}] planner: analysis complete", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[{}] Planner analysis", config.mock_tag),
        &format!(
            "Mock planner run for workflow validation.\n\nMock tag: `{}`\nRequirement: `{}`",
            config.mock_tag,
            requirement_path.display()
        ),
    )?;

    println!("Mock planner: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}
