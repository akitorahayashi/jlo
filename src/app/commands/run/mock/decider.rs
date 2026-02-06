use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::app::commands::run::RunOptions;
use crate::app::commands::run::mock::identity::generate_mock_id;
use crate::domain::{AppError, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Execute mock deciders.
pub fn execute_mock_deciders<G, H, W>(
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
    let branch_name = config.branch_name(Layer::Deciders, &timestamp)?;

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
    std::fs::create_dir_all(&decided_dir)?;
    std::fs::create_dir_all(&issues_dir)?;

    // Create two mock issues: one for planner, one for implementer
    let label = config.issue_labels.first().cloned().ok_or_else(|| {
        AppError::Validation("No issue labels available for mock decider".to_string())
    })?;
    let label_dir = issues_dir.join(&label);
    std::fs::create_dir_all(&label_dir)?;

    let mock_issue_template = include_str!("assets/decider_issue.yml");

    // Move any mock pending events to decided first
    let mut moved_src_files: Vec<PathBuf> = Vec::new();
    if pending_dir.exists() {
        for entry in std::fs::read_dir(&pending_dir)? {
            let entry = entry?;
            let path = entry.path();
            if mock_event_id_from_path(&path, &config.mock_tag).is_some() {
                let dest = decided_dir.join(path.file_name().ok_or_else(|| {
                    AppError::Validation(format!(
                        "Pending event missing filename: {}",
                        path.display()
                    ))
                })?);
                std::fs::rename(&path, &dest)?;
                moved_src_files.push(path);
            }
        }
    }

    let decided_mock_files = list_mock_decided_files(&decided_dir, &config.mock_tag)?;
    let source_event_ids: Vec<String> = decided_mock_files
        .iter()
        .filter_map(|path| mock_event_id_from_path(path, &config.mock_tag))
        .collect();

    if source_event_ids.len() < 2 {
        return Err(AppError::Validation(format!(
            "Mock decider requires at least 2 decided events for tag '{}', found {}",
            config.mock_tag,
            source_event_ids.len()
        )));
    }

    let planner_event_id = source_event_ids[0].clone();
    let impl_event_id = source_event_ids[1].clone();

    // Issue 1: requires deep analysis (for planner)
    let planner_issue_id = generate_mock_id();
    let planner_issue_file = label_dir.join(format!("mock-planner-{}.yml", config.mock_tag));
    let planner_issue_content = mock_issue_template
        .replace("mock01", &planner_issue_id)
        .replace("test-tag", &config.mock_tag)
        .replace("event1", &planner_event_id)
        .replace(
            "requires_deep_analysis: false",
            "requires_deep_analysis: true\ndeep_analysis_reason: \"Mock issue requires architectural analysis-for-workflow-validation\"",
        )
        .replace(
            "Mock issue for workflow validation",
            "Mock issue requiring deep analysis",
        )
        .replace("medium", "high"); // Make it high priority for planner

    workspace.write_file(
        planner_issue_file
            .to_str()
            .ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &planner_issue_content,
    )?;

    // Issue 2: ready for implementer
    let impl_issue_id = generate_mock_id();
    let impl_issue_file = label_dir.join(format!("mock-impl-{}.yml", config.mock_tag));
    let impl_issue_content = mock_issue_template
        .replace("mock01", &impl_issue_id)
        .replace("test-tag", &config.mock_tag)
        .replace("event1", &impl_event_id)
        .replace("Mock issue for workflow validation", "Mock issue ready for implementation");

    workspace.write_file(
        impl_issue_file.to_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?,
        &impl_issue_content,
    )?;

    // Ensure all tag-matched decided events have issue_id.
    // Extra events (e.g., workflow re-run with same mock tag) are attached to implementer issue.
    for decided_file in &decided_mock_files {
        if let Some(event_id) = mock_event_id_from_path(decided_file, &config.mock_tag) {
            let assigned_issue_id =
                if event_id == planner_event_id { &planner_issue_id } else { &impl_issue_id };

            let content = match std::fs::read_to_string(decided_file) {
                Ok(content) => content,
                Err(err) => {
                    println!(
                        "::warning::Failed to read decided event file {}: {}",
                        decided_file.display(),
                        err
                    );
                    continue;
                }
            };

            let mut yaml_value: serde_yaml::Value = match serde_yaml::from_str(&content) {
                Ok(value) => value,
                Err(err) => {
                    println!(
                        "::warning::Failed to parse decided event file {} as YAML: {}",
                        decided_file.display(),
                        err
                    );
                    continue;
                }
            };

            let Some(mapping) = yaml_value.as_mapping_mut() else {
                println!(
                    "::warning::Decided event file is not a YAML mapping: {}",
                    decided_file.display()
                );
                continue;
            };

            mapping.insert(
                serde_yaml::Value::String("issue_id".to_string()),
                serde_yaml::Value::String(assigned_issue_id.to_string()),
            );

            let updated_content = match serde_yaml::to_string(&yaml_value) {
                Ok(value) => value,
                Err(err) => {
                    println!(
                        "::warning::Failed to render decided event YAML {}: {}",
                        decided_file.display(),
                        err
                    );
                    continue;
                }
            };

            if let Err(err) = std::fs::write(decided_file, updated_content) {
                println!(
                    "::warning::Failed to write decided event file {}: {}",
                    decided_file.display(),
                    err
                );
            }
        }
    }

    // Commit and push (include moved/deleted files and decided updates)
    let mut files: Vec<&Path> = vec![planner_issue_file.as_path(), impl_issue_file.as_path()];
    for f in &decided_mock_files {
        files.push(f.as_path());
    }
    for f in &moved_src_files {
        files.push(f.as_path());
    }
    git.commit_files(&format!("[mock-{}] decider: mock issues", config.mock_tag), &files)?;
    git.push_branch(&branch_name, false)?;

    // Create PR
    let pr = github.create_pull_request(
        &branch_name,
        &config.jules_branch,
        &format!("[mock-{}] Decider triage", config.mock_tag),
        &format!("Mock decider run for workflow validation.\n\nMock tag: `{}`\nWorkstream: `{}`\n\nCreated issues:\n- `{}` (requires analysis)\n- `{}` (ready for impl)",
            config.mock_tag, workstream, planner_issue_id, impl_issue_id),
    )?;

    println!("Mock deciders: created PR #{} ({})", pr.number, pr.url);

    Ok(MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    })
}

fn mock_event_id_from_path(path: &Path, mock_tag: &str) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let prefix = format!("mock-{}-", mock_tag);
    file_name.strip_prefix(&prefix)?.strip_suffix(".yml").map(ToString::to_string)
}

fn list_mock_decided_files(decided_dir: &Path, mock_tag: &str) -> Result<Vec<PathBuf>, AppError> {
    if !decided_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(decided_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| mock_event_id_from_path(path, mock_tag).is_some())
        .collect();

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{list_mock_decided_files, mock_event_id_from_path};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_mock_event_id_from_path() {
        let mock_tag = "mock-run-123";
        let valid_path = std::path::Path::new("mock-mock-run-123-a1b2c3.yml");
        let invalid_path = std::path::Path::new("mock-other-tag-a1b2c3.yml");

        assert_eq!(mock_event_id_from_path(valid_path, mock_tag), Some("a1b2c3".to_string()));
        assert_eq!(mock_event_id_from_path(invalid_path, mock_tag), None);
    }

    #[test]
    fn lists_only_tagged_decided_files_in_sorted_order() {
        let dir = tempdir().expect("tempdir");
        let decided_dir = dir.path().join("decided");
        fs::create_dir_all(&decided_dir).expect("mkdir");

        fs::write(decided_dir.join("mock-mock-run-123-bbbbbb.yml"), "id: bbbbbb\n").expect("write");
        fs::write(decided_dir.join("mock-mock-run-123-aaaaaa.yml"), "id: aaaaaa\n").expect("write");
        fs::write(decided_dir.join("mock-other-run-cccccc.yml"), "id: cccccc\n").expect("write");
        fs::write(decided_dir.join("notes.txt"), "ignored\n").expect("write");

        let files = list_mock_decided_files(&decided_dir, "mock-run-123").expect("list");

        assert_eq!(files.len(), 2);
        assert!(files[0].to_string_lossy().ends_with("mock-mock-run-123-aaaaaa.yml"));
        assert!(files[1].to_string_lossy().ends_with("mock-mock-run-123-bbbbbb.yml"));
    }
}
