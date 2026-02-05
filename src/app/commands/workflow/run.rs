//! Workflow run command implementation.
//!
//! Runs a layer sequentially using a matrix input and returns wait-gating metadata.
//! This command orchestrates actual execution by calling `jlo run` for each matrix entry.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::domain::{AppError, Layer};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;

/// Options for workflow run command.
#[derive(Debug, Clone)]
pub struct WorkflowRunOptions {
    /// Target layer.
    pub layer: Layer,
    /// Matrix JSON input (from matrix commands).
    pub matrix_json: Option<MatrixInput>,
    /// Target branch for implementers.
    #[allow(dead_code)]
    pub target_branch: Option<String>,
    /// Run in mock mode.
    pub mock: bool,
}

/// Input matrix structure.
#[derive(Debug, Clone, Deserialize)]
pub struct MatrixInput {
    /// Matrix include entries.
    pub include: Vec<serde_json::Value>,
}

/// Output of workflow run command.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRunOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Expected number of PRs to wait for.
    pub expected_count: usize,
    /// Timestamp when run started (RFC3339 UTC).
    pub run_started_at: String,
    /// Whether this was a mock run.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub mock: bool,
    /// Mock tag (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_tag: Option<String>,
    /// Mock PR numbers (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_pr_numbers: Option<Vec<u64>>,
    /// Mock branches (only in mock mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_branches: Option<Vec<String>>,
}

/// Execute workflow run command.
pub fn execute(options: WorkflowRunOptions) -> Result<WorkflowRunOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let run_started_at = Utc::now().to_rfc3339();

    // Mock mode configuration
    let mock_tag = if options.mock {
        let tag = std::env::var("JULES_MOCK_TAG").map_err(|_| {
            AppError::Validation(
                "Mock mode requires JULES_MOCK_TAG environment variable".to_string(),
            )
        })?;

        if !tag.contains("mock") {
            return Err(AppError::Validation(
                "JULES_MOCK_TAG must contain 'mock' substring".to_string(),
            ));
        }
        Some(tag)
    } else {
        None
    };

    // Execute runs based on layer
    let run_results = execute_layer_runs(&options, mock_tag.as_deref())?;

    // Calculate expected count (matches run_results.run_count for verification)
    let expected_count = run_results.run_count;

    Ok(WorkflowRunOutput {
        schema_version: 1,
        expected_count,
        run_started_at,
        mock_tag,
        mock_pr_numbers: run_results.mock_pr_numbers,
        mock_branches: run_results.mock_branches,
        mock: options.mock,
    })
}

/// Results from running a layer.
struct RunResults {
    run_count: usize,
    mock_pr_numbers: Option<Vec<u64>>,
    mock_branches: Option<Vec<String>>,
}

/// Execute runs for a layer based on matrix input.
fn execute_layer_runs(
    options: &WorkflowRunOptions,
    mock_tag: Option<&str>,
) -> Result<RunResults, AppError> {
    match options.layer {
        Layer::Narrators => execute_narrator_run(mock_tag),
        Layer::Observers | Layer::Deciders => execute_multi_role_runs(options, mock_tag),
        Layer::Planners | Layer::Implementers => execute_issue_runs(options, mock_tag),
    }
}

/// Execute narrator run (single entry, no matrix).
fn execute_narrator_run(mock_tag: Option<&str>) -> Result<RunResults, AppError> {
    let mut cmd = Command::new("jlo");
    cmd.arg("run").arg("narrator");

    if let Some(tag) = mock_tag {
        cmd.arg("--mock").env("JULES_MOCK_TAG", tag);
    }

    eprintln!("Executing: jlo run narrator{}", if mock_tag.is_some() { " --mock" } else { "" });

    let output = cmd.output().map_err(|e| AppError::ExternalToolError {
        tool: "jlo".to_string(),
        error: format!("Failed to execute jlo run: {}", e),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::ExternalToolError {
            tool: "jlo".to_string(),
            error: format!("jlo run narrator failed: {}", stderr),
        });
    }

    // Parse output for mock PR numbers and branches
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (mock_pr_numbers, mock_branches) = parse_mock_output(&stdout);

    Ok(RunResults { run_count: 1, mock_pr_numbers, mock_branches })
}

/// Execute runs for multi-role layers (observers, deciders).
fn execute_multi_role_runs(
    options: &WorkflowRunOptions,
    mock_tag: Option<&str>,
) -> Result<RunResults, AppError> {
    let matrix = options.matrix_json.as_ref().ok_or_else(|| {
        AppError::MissingArgument(format!(
            "Matrix JSON is required for layer '{}'",
            options.layer.dir_name()
        ))
    })?;

    let mut all_pr_numbers = Vec::new();
    let mut all_branches = Vec::new();
    let mock_flag_str = if mock_tag.is_some() { " --mock" } else { "" };

    // For deciders, deduplicate by workstream (one run per workstream)
    // For observers, run each entry
    let deduped_entries: Vec<&serde_json::Value>;
    let entries: &[&serde_json::Value] = if options.layer == Layer::Deciders {
        let mut seen = std::collections::HashSet::new();
        deduped_entries = matrix
            .include
            .iter()
            .filter(|entry| {
                entry
                    .get("workstream")
                    .and_then(|v| v.as_str())
                    .map(|ws| seen.insert(ws.to_string()))
                    .unwrap_or(false)
            })
            .collect();
        &deduped_entries
    } else {
        deduped_entries = matrix.include.iter().collect();
        &deduped_entries
    };

    for entry in entries {
        let workstream = entry.get("workstream").and_then(|v| v.as_str()).ok_or_else(|| {
            AppError::Validation("Matrix entry missing 'workstream' field".to_string())
        })?;

        // For observers, also get role
        let role = if options.layer == Layer::Observers {
            Some(entry.get("role").and_then(|v| v.as_str()).ok_or_else(|| {
                AppError::Validation("Observer matrix entry missing 'role' field".to_string())
            })?)
        } else {
            None
        };

        let mut cmd = Command::new("jlo");
        cmd.arg("run").arg(options.layer.dir_name());
        cmd.arg("--workstream").arg(workstream);
        cmd.arg("--scheduled");

        if let Some(role) = role {
            cmd.arg("--role").arg(role);
        }

        if let Some(tag) = mock_tag {
            cmd.arg("--mock").env("JULES_MOCK_TAG", tag);
        }

        let cmd_str = if let Some(role) = role {
            format!(
                "jlo run {} --workstream {} --role {} --scheduled{}",
                options.layer.dir_name(),
                workstream,
                role,
                mock_flag_str
            )
        } else {
            format!(
                "jlo run {} --workstream {} --scheduled{}",
                options.layer.dir_name(),
                workstream,
                mock_flag_str
            )
        };
        eprintln!("Executing: {}", cmd_str);

        let output = cmd.output().map_err(|e| AppError::ExternalToolError {
            tool: "jlo".to_string(),
            error: format!("Failed to execute jlo run: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::ExternalToolError {
                tool: "jlo".to_string(),
                error: format!("jlo run {} failed: {}", options.layer.dir_name(), stderr),
            });
        }

        // Parse output for mock PR numbers and branches
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (pr_nums, branches) = parse_mock_output(&stdout);
        if let Some(nums) = pr_nums {
            all_pr_numbers.extend(nums);
        }
        if let Some(brs) = branches {
            all_branches.extend(brs);
        }
    }

    Ok(RunResults {
        run_count: entries.len(),
        mock_pr_numbers: if all_pr_numbers.is_empty() { None } else { Some(all_pr_numbers) },
        mock_branches: if all_branches.is_empty() { None } else { Some(all_branches) },
    })
}

/// Execute runs for issue-based layers (planners, implementers).
fn execute_issue_runs(
    options: &WorkflowRunOptions,
    mock_tag: Option<&str>,
) -> Result<RunResults, AppError> {
    let matrix = options.matrix_json.as_ref().ok_or_else(|| {
        AppError::MissingArgument(format!(
            "Matrix JSON is required for layer '{}'",
            options.layer.dir_name()
        ))
    })?;

    let mut all_pr_numbers = Vec::new();
    let mut all_branches = Vec::new();

    for entry in &matrix.include {
        let issue = entry.get("issue").and_then(|v| v.as_str()).ok_or_else(|| {
            AppError::Validation("Matrix entry missing 'issue' field".to_string())
        })?;

        let mut cmd = Command::new("jlo");
        cmd.arg("run").arg(options.layer.dir_name());
        cmd.arg(issue); // Positional argument

        if let Some(tag) = mock_tag {
            cmd.arg("--mock").env("JULES_MOCK_TAG", tag);
        }

        eprintln!(
            "Executing: jlo run {} {}{}",
            options.layer.dir_name(),
            issue,
            if mock_tag.is_some() { " --mock" } else { "" }
        );

        let output = cmd.output().map_err(|e| AppError::ExternalToolError {
            tool: "jlo".to_string(),
            error: format!("Failed to execute jlo run: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::ExternalToolError {
                tool: "jlo".to_string(),
                error: format!("jlo run {} failed: {}", options.layer.dir_name(), stderr),
            });
        }

        // Parse output for mock PR numbers and branches
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (pr_nums, branches) = parse_mock_output(&stdout);
        if let Some(nums) = pr_nums {
            all_pr_numbers.extend(nums);
        }
        if let Some(brs) = branches {
            all_branches.extend(brs);
        }
    }

    Ok(RunResults {
        run_count: matrix.include.len(),
        mock_pr_numbers: if all_pr_numbers.is_empty() { None } else { Some(all_pr_numbers) },
        mock_branches: if all_branches.is_empty() { None } else { Some(all_branches) },
    })
}

/// Parse mock output from jlo run stdout to extract PR numbers and branches.
fn parse_mock_output(stdout: &str) -> (Option<Vec<u64>>, Option<Vec<String>>) {
    // Mock output format includes lines like:
    // PR: https://github.com/owner/repo/pull/123
    // Branch: jules/mock-tag/...
    let mut pr_numbers = Vec::new();
    let mut branches = Vec::new();

    for line in stdout.lines() {
        if let Some(pr_url) = line.strip_prefix("PR: ") {
            // Extract PR number from URL
            if let Some(num) = pr_url.rsplit('/').next().and_then(|s| s.parse::<u64>().ok()) {
                pr_numbers.push(num);
            }
        } else if let Some(branch) = line.strip_prefix("Branch: ") {
            branches.push(branch.trim().to_string());
        }
    }

    let pr_nums = if pr_numbers.is_empty() { None } else { Some(pr_numbers) };
    let brs = if branches.is_empty() { None } else { Some(branches) };
    (pr_nums, brs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mock_output_extracts_pr_numbers() {
        let output = r#"
Running mock narrator...
PR: https://github.com/owner/repo/pull/123
Branch: jules/mock-tag-123/narrator
Done.
"#;
        let (pr_nums, branches) = parse_mock_output(output);
        assert_eq!(pr_nums, Some(vec![123]));
        assert_eq!(branches, Some(vec!["jules/mock-tag-123/narrator".to_string()]));
    }

    #[test]
    fn parse_mock_output_handles_multiple_prs() {
        let output = r#"
PR: https://github.com/owner/repo/pull/1
Branch: jules/mock/branch1
PR: https://github.com/owner/repo/pull/2
Branch: jules/mock/branch2
"#;
        let (pr_nums, branches) = parse_mock_output(output);
        assert_eq!(pr_nums, Some(vec![1, 2]));
        assert_eq!(branches.as_ref().map(|b| b.len()), Some(2));
    }

    #[test]
    fn parse_mock_output_returns_none_when_empty() {
        let output = "No mock output";
        let (pr_nums, branches) = parse_mock_output(output);
        assert_eq!(pr_nums, None);
        assert_eq!(branches, None);
    }
}
