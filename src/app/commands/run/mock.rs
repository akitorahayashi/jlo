//! Mock execution for workflow validation without Jules API.
//!
//! This module provides mock implementations of agent execution that perform
//! real git and GitHub operations without calling the Jules API. This enables
//! end-to-end validation of the workflow kit.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use chrono::Utc;

use super::RunOptions;
use super::RunResult;
use super::config::load_config;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute in mock mode.
pub fn execute<G, H, W>(
    jules_path: &Path,
    options: &RunOptions,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<RunResult, AppError>
where
    G: GitPort,
    H: GitHubPort,
    W: WorkspaceStore,
{
    // Validate mock prerequisites
    validate_mock_prerequisites(options)?;

    // Load mock configuration from workspace
    let mock_config = load_mock_config(jules_path, options, workspace)?;

    // Execute layer-specific mock behavior
    let output = match options.layer {
        Layer::Narrators => execute_mock_narrator(jules_path, &mock_config, git, github, workspace),
        Layer::Observers => {
            execute_mock_observers(jules_path, options, &mock_config, git, github, workspace)
        }
        Layer::Deciders => {
            execute_mock_deciders(jules_path, options, &mock_config, git, github, workspace)
        }
        Layer::Planners => {
            execute_mock_planners(jules_path, options, &mock_config, git, github, workspace)
        }
        Layer::Implementers => {
            execute_mock_implementers(jules_path, options, &mock_config, git, github, workspace)
        }
    }?;

    // Write outputs
    if std::env::var("GITHUB_OUTPUT").is_ok() {
        output.write_github_output().map_err(|e| {
            AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
        })?;
    } else {
        output.print_local();
    }

    Ok(RunResult {
        roles: vec![options.layer.dir_name().to_string()],
        dry_run: false,
        sessions: vec![], // No Jules sessions in mock mode
    })
}

/// Validate prerequisites for mock mode.
fn validate_mock_prerequisites(_options: &RunOptions) -> Result<(), AppError> {
    // Check for GH_TOKEN
    if std::env::var("GH_TOKEN").is_err() {
        return Err(AppError::MissingArgument(
            "Mock mode requires GH_TOKEN environment variable to be set".to_string(),
        ));
    }

    // Check for required tools
    if std::process::Command::new("git").arg("--version").output().is_err() {
        return Err(AppError::ExternalToolError {
            tool: "git".to_string(),
            error: "git is required for mock mode but not found in PATH".to_string(),
        });
    }

    if std::process::Command::new("gh").arg("--version").output().is_err() {
        return Err(AppError::ExternalToolError {
            tool: "gh".to_string(),
            error: "gh CLI is required for mock mode but not found in PATH".to_string(),
        });
    }

    Ok(())
}

/// Load mock configuration from workspace files.
fn load_mock_config<W: WorkspaceStore>(
    jules_path: &Path,
    _options: &RunOptions,
    workspace: &W,
) -> Result<MockConfig, AppError> {
    // Load run config for branch settings
    let run_config = load_config(jules_path)?;

    // Load branch prefixes from contracts.yml files
    let mut branch_prefixes = HashMap::new();
    for layer in
        [Layer::Narrators, Layer::Observers, Layer::Deciders, Layer::Planners, Layer::Implementers]
    {
        let contracts_path = jules_path.join("roles").join(layer.dir_name()).join("contracts.yml");

        if let Ok(content) = workspace.read_file(
            contracts_path
                .to_str()
                .ok_or_else(|| AppError::Validation("Invalid contracts path".to_string()))?,
        ) && let Some(prefix) = extract_branch_prefix(&content)
        {
            branch_prefixes.insert(layer, prefix);
        }
    }

    // Load issue labels from github-labels.json
    let labels_path = jules_path.join("github-labels.json");
    let issue_labels = if let Ok(content) = workspace.read_file(
        labels_path
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid labels path".to_string()))?,
    ) {
        extract_issue_labels(&content)?
    } else {
        vec!["bugs".to_string(), "feats".to_string(), "refacts".to_string()]
    };

    // Generate scope if not provided
    // Generate scope: env var -> CI default -> local default
    let scope = std::env::var("JULES_MOCK_SCOPE").ok().unwrap_or_else(|| {
        let prefix = if std::env::var("GITHUB_ACTIONS").is_ok() { "ci" } else { "local" };
        format!("{}-{}", prefix, Utc::now().format("%Y%m%d%H%M%S"))
    });

    Ok(MockConfig {
        scope,
        branch_prefixes,
        default_branch: run_config.run.default_branch,
        jules_branch: run_config.run.jules_branch,
        issue_labels,
    })
}

/// Extract branch_prefix from contracts.yml content.
fn extract_branch_prefix(content: &str) -> Option<String> {
    // Simple YAML parsing for branch_prefix
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("branch_prefix:") {
            let value = line.trim_start_matches("branch_prefix:").trim();
            // Remove quotes if present
            let value = value.trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Extract issue labels from github-labels.json content.
fn extract_issue_labels(content: &str) -> Result<Vec<String>, AppError> {
    let json: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        AppError::ParseError { what: "github-labels.json".to_string(), details: e.to_string() }
    })?;

    let labels = json
        .get("issue_labels")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    Ok(labels)
}

/// Execute mock narrator.
fn execute_mock_narrator<G, H, W>(
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
    let branch_name = config.branch_name(Layer::Narrators, &timestamp);

    println!("Mock narrator: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create mock changes file
    let changes_dir = jules_path.join("changes");
    let changes_file = changes_dir.join("latest.yml");

    let changes_content = format!(
        r#"# Mock changes generated by jlo --mock
# Scope: {}
generated_at: "{}"
mock: true
summary: "Mock narrator run for workflow validation"
changes: []
"#,
        config.scope,
        Utc::now().to_rfc3339()
    );

    workspace.write_file(
        changes_file.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &changes_content,
    )?;

    // Commit and push
    let files: Vec<&Path> = vec![changes_file.as_path()];
    git.commit_files(&format!("[mock-{}] narrator: mock changes", config.scope), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Narrator changes", config.scope),
        &format!("Mock narrator run for workflow validation.\n\nScope: `{}`", config.scope),
    )?;

    // Enable auto-merge for .jules/-only PRs
    github.enable_auto_merge(pr.number)?;

    println!("Mock narrator: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_scope: config.scope.clone(),
    })
}

/// Execute mock observers.
fn execute_mock_observers<G, H, W>(
    jules_path: &Path,
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
    let workstream = options.workstream.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Workstream is required for observers".to_string())
    })?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Observers, &timestamp);

    println!("Mock observers: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    // Create mock event file
    let event_id = generate_mock_id();
    let events_dir = jules_path
        .join("workstreams")
        .join(workstream)
        .join("exchange")
        .join("events")
        .join("pending");

    let event_file = events_dir.join(format!("mock-{}-{}.yml", config.scope, event_id));

    let event_content = format!(
        r#"id: "{}"
type: observation
title: "Mock observation for workflow validation"
description: |
  This is a mock observation created by `jlo run --mock` for workflow-kit validation.
  Scope: {}
author_role: mock
created_at: "{}"
mock: true
"#,
        event_id,
        config.scope,
        Utc::now().to_rfc3339()
    );

    // Ensure directory exists
    std::fs::create_dir_all(&events_dir).map_err(AppError::Io)?;

    workspace.write_file(
        event_file.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &event_content,
    )?;

    // Commit and push
    let files: Vec<&Path> = vec![event_file.as_path()];
    git.commit_files(&format!("[mock-{}] observer: mock event", config.scope), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Observer findings", config.scope),
        &format!(
            "Mock observer run for workflow validation.\n\nScope: `{}`\nWorkstream: `{}`",
            config.scope, workstream
        ),
    )?;

    // Enable auto-merge
    github.enable_auto_merge(pr.number)?;

    println!("Mock observers: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_scope: config.scope.clone(),
    })
}

/// Execute mock deciders.
fn execute_mock_deciders<G, H, W>(
    jules_path: &Path,
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
    let workstream = options.workstream.as_deref().ok_or_else(|| {
        AppError::MissingArgument("Workstream is required for deciders".to_string())
    })?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Deciders, &timestamp);

    println!("Mock deciders: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    git.fetch("origin")?;
    git.checkout_branch(&format!("origin/{}", config.jules_branch), false)?;
    git.checkout_branch(&branch_name, true)?;

    let exchange_dir = jules_path.join("workstreams").join(workstream).join("exchange");

    // Find and process pending events
    let pending_dir = exchange_dir.join("events").join("pending");
    let decided_dir = exchange_dir.join("events").join("decided");
    let issues_dir = exchange_dir.join("issues");

    // Ensure directories exist
    std::fs::create_dir_all(&decided_dir).ok();
    std::fs::create_dir_all(&issues_dir).ok();

    // Create two mock issues: one for planner, one for implementer
    let label = config.issue_labels.first().cloned().unwrap_or_else(|| "bugs".to_string());
    let label_dir = issues_dir.join(&label);
    std::fs::create_dir_all(&label_dir).ok();

    // Issue 1: requires deep analysis (for planner)
    let planner_issue_id = generate_mock_id();
    let planner_issue_file = label_dir.join(format!("mock-planner-{}.yml", config.scope));
    let planner_issue_content = format!(
        r#"id: "{}"
label: "{}"
title: "Mock issue requiring deep analysis"
description: |
  This mock issue requires deep analysis and will be processed by the planner.
  Scope: {}
requires_deep_analysis: true
deep_analysis_reason: "Complex system impact requires detailed planning"
source_events: ["mock-event-1"]
created_at: "{}"
mock: true
"#,
        planner_issue_id,
        label,
        config.scope,
        Utc::now().to_rfc3339()
    );

    workspace.write_file(
        planner_issue_file
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &planner_issue_content,
    )?;

    // Issue 2: ready for implementer
    let impl_issue_id = generate_mock_id();
    let impl_issue_file = label_dir.join(format!("mock-impl-{}.yml", config.scope));
    let impl_issue_content = format!(
        r#"id: "{}"
label: "{}"
title: "Mock issue ready for implementation"
description: |
  This mock issue is ready for implementation.
  Scope: {}
requires_deep_analysis: false
source_events: ["mock-event-2"]
verification_commands:
  - "echo 'Mock verification'"
created_at: "{}"
mock: true
"#,
        impl_issue_id,
        label,
        config.scope,
        Utc::now().to_rfc3339()
    );

    workspace.write_file(
        impl_issue_file.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &impl_issue_content,
    )?;

    // Move any mock pending events to decided
    if pending_dir.exists() {
        for entry in std::fs::read_dir(&pending_dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path
                .file_name()
                .map(|n| n.to_string_lossy().contains(&config.scope))
                .unwrap_or(false)
            {
                let dest = decided_dir.join(path.file_name().unwrap());
                std::fs::rename(&path, &dest).ok();
            }
        }
    }

    // Commit and push
    let files: Vec<&Path> = vec![planner_issue_file.as_path(), impl_issue_file.as_path()];
    git.commit_files(&format!("[mock-{}] decider: mock issues", config.scope), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Decider triage", config.scope),
        &format!("Mock decider run for workflow validation.\n\nScope: `{}`\nWorkstream: `{}`\n\nCreated issues:\n- `{}` (requires analysis)\n- `{}` (ready for impl)", 
            config.scope, workstream, planner_issue_id, impl_issue_id),
    )?;

    // Enable auto-merge
    github.enable_auto_merge(pr.number)?;

    println!("Mock deciders: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_scope: config.scope.clone(),
    })
}

/// Execute mock planners.
fn execute_mock_planners<G, H, W>(
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
    let branch_name = config.branch_name(Layer::Planners, &timestamp);

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
  Scope: {}
  
  ## Impact Analysis
  - Mock impact area 1
  - Mock impact area 2
  
  ## Implementation Notes
  - No actual analysis performed (mock mode)
"#,
            Utc::now().to_rfc3339(),
            config.scope
        );

    workspace.write_file(issue_path_str, &updated_content)?;

    // Commit and push
    let files: Vec<&Path> = vec![issue_path.as_path()];
    git.commit_files(&format!("[mock-{}] planner: analysis complete", config.scope), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Planner analysis", config.scope),
        &format!(
            "Mock planner run for workflow validation.\n\nScope: `{}`\nIssue: `{}`",
            config.scope,
            issue_path.display()
        ),
    )?;

    // Enable auto-merge
    github.enable_auto_merge(pr.number)?;

    println!("Mock planners: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_scope: config.scope.clone(),
    })
}

/// Execute mock implementers.
fn execute_mock_implementers<G, H, W>(
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

/// Generate a 6-character mock ID.
fn generate_mock_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{:06x}", (timestamp % 0xFFFFFF) as u32)
}

/// Wait for a mock PR to be merged.
#[allow(dead_code)]
pub fn wait_for_mock_pr_merge<H: GitHubPort>(
    github: &H,
    pr_number: u64,
    timeout_minutes: u64,
) -> Result<(), AppError> {
    println!("Waiting for PR #{} to merge (timeout: {} min)...", pr_number, timeout_minutes);
    github.wait_for_merge(pr_number, Duration::from_secs(timeout_minutes * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_branch_prefix() {
        let content = r#"
layer: observers
branch_prefix: jules-observer-
constraints:
  - Do NOT write to issues/
"#;
        assert_eq!(extract_branch_prefix(content), Some("jules-observer-".to_string()));
    }

    #[test]
    fn test_extract_branch_prefix_with_quotes() {
        let content = r#"branch_prefix: "jules-test-""#;
        assert_eq!(extract_branch_prefix(content), Some("jules-test-".to_string()));
    }

    #[test]
    fn test_extract_issue_labels() {
        let content = r#"{
            "issue_labels": {
                "bugs": {"color": "d73a4a"},
                "feats": {"color": "ff6600"}
            }
        }"#;
        let labels = extract_issue_labels(content).unwrap();
        assert!(labels.contains(&"bugs".to_string()));
        assert!(labels.contains(&"feats".to_string()));
    }

    #[test]
    fn test_generate_mock_id() {
        let id1 = generate_mock_id();
        let id2 = generate_mock_id();
        assert_eq!(id1.len(), 6);
        assert_eq!(id2.len(), 6);
        // IDs should be different (very high probability)
        // Note: This could theoretically fail if called in same nanosecond
    }

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
