use std::collections::HashSet;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};
use serde::Deserialize;

use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, IoErrorKind, Layer, MockConfig, MockOutput};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};

/// Mock assets embedded in the binary.
pub static MOCK_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/assets/mock");

/// Write outputs to GITHUB_OUTPUT file if set.
pub fn write_github_output(output: &MockOutput) -> std::io::Result<()> {
    if let Ok(output_file) = std::env::var("GITHUB_OUTPUT") {
        use std::io::Write;
        ensure_single_line_output_value("mock_branch", &output.mock_branch)?;
        ensure_single_line_output_value("mock_pr_url", &output.mock_pr_url)?;
        ensure_single_line_output_value("mock_tag", &output.mock_tag)?;
        let mut file = std::fs::OpenOptions::new().append(true).open(&output_file)?;
        writeln!(file, "mock_branch={}", output.mock_branch)?;
        writeln!(file, "mock_pr_number={}", output.mock_pr_number)?;
        writeln!(file, "mock_pr_url={}", output.mock_pr_url)?;
        writeln!(file, "mock_tag={}", output.mock_tag)?;
    }
    Ok(())
}

fn ensure_single_line_output_value(name: &str, value: &str) -> std::io::Result<()> {
    if value.contains('\n') || value.contains('\r') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Output value '{}' contains a newline and cannot be written safely", name),
        ));
    }
    Ok(())
}

/// Print outputs in grep-friendly format for local use.
pub fn print_local(output: &MockOutput) {
    println!("MOCK_BRANCH={}", output.mock_branch);
    println!("MOCK_PR_NUMBER={}", output.mock_pr_number);
    println!("MOCK_PR_URL={}", output.mock_pr_url);
    println!("MOCK_TAG={}", output.mock_tag);
}

/// Generate a 6-character mock ID.
pub fn generate_mock_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{:06x}", (timestamp % 0xFFFFFF) as u32)
}

/// Parse mock event ID from filename.
pub fn mock_event_id_from_path(path: &Path, mock_tag: &str) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let prefix = format!("{}-", mock_tag);
    file_name.strip_prefix(&prefix)?.strip_suffix(".yml").map(ToString::to_string)
}

/// List files in directory matching the mock tag pattern.
pub fn list_mock_tagged_files<W: WorkspaceStore + ?Sized>(
    workspace: &W,
    dir: &Path,
    mock_tag: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let dir_str = dir.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!("Invalid directory path: {}", dir.display()))
    })?;

    let entries = match workspace.list_dir(dir_str) {
        Ok(entries) => entries,
        Err(AppError::Io { kind: IoErrorKind::NotFound, .. }) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let mut files: Vec<PathBuf> = entries
        .into_iter()
        .filter(|path| !workspace.is_dir(&path.to_string_lossy()))
        .filter(|path| mock_event_id_from_path(path, mock_tag).is_some())
        .collect();

    files.sort();
    Ok(files)
}

/// Service for executing mock workflows.
pub struct MockExecutionService<'a, G: ?Sized, H: ?Sized, W: ?Sized> {
    #[allow(dead_code)]
    pub jules_path: &'a Path,
    #[allow(dead_code)]
    pub config: &'a MockConfig,
    pub git: &'a G,
    pub github: &'a H,
    #[allow(dead_code)]
    pub workspace: &'a W,
}

impl<'a, G, H, W> MockExecutionService<'a, G, H, W>
where
    G: GitPort + ?Sized,
    H: GitHubPort + ?Sized,
    W: WorkspaceStore + ?Sized,
{
    pub fn new(
        jules_path: &'a Path,
        config: &'a MockConfig,
        git: &'a G,
        github: &'a H,
        workspace: &'a W,
    ) -> Self {
        Self { jules_path, config, git, github, workspace }
    }

    /// Fetch origin and checkout a base branch (detached HEAD).
    pub fn fetch_and_checkout_base(&self, base_branch: &str) -> Result<(), AppError> {
        self.git.fetch("origin")?;
        self.git.checkout_branch(&format!("origin/{}", base_branch), false)?;
        Ok(())
    }

    /// Checkout a new branch from the current HEAD.
    pub fn checkout_new_branch(&self, branch: &str) -> Result<(), AppError> {
        self.git.checkout_branch(branch, true)
    }

    /// Commit files and push the current branch.
    pub fn commit_and_push(
        &self,
        message: &str,
        files: &[&Path],
        branch: &str,
    ) -> Result<(), AppError> {
        self.git.commit_files(message, files)?;
        self.git.push_branch(branch, false)?;
        Ok(())
    }

    /// Create a pull request.
    pub fn create_pr(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<crate::ports::PullRequestInfo, AppError> {
        self.github.create_pull_request(head, base, title, body)
    }

    /// Write mock output to GITHUB_OUTPUT or stdout.
    pub fn finish(&self, output: &MockOutput) -> Result<(), AppError> {
        if std::env::var("GITHUB_OUTPUT").is_ok() {
            write_github_output(output).map_err(|e| {
                AppError::InternalError(format!("Failed to write GITHUB_OUTPUT: {}", e))
            })?;
        } else {
            print_local(output);
        }
        Ok(())
    }
}

pub fn execute_decider_mock<G, H, W>(
    jules_path: &Path,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort + ?Sized,
    H: GitHubPort + ?Sized,
    W: WorkspaceStore,
{
    let service = MockExecutionService::new(jules_path, config, git, github, workspace);

    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let branch_name = config.branch_name(Layer::Decider, &timestamp)?;

    println!("Mock decider: creating branch {}", branch_name);

    // Fetch and checkout from jules branch
    service.fetch_and_checkout_base(&config.jules_branch)?;
    service.checkout_new_branch(&branch_name)?;

    // Find and process pending events
    let pending_dir = jules::events_pending_dir(jules_path);
    let decided_dir = jules::events_decided_dir(jules_path);
    let requirements_dir = jules::requirements_dir(jules_path);

    // Ensure directories exist.
    workspace.create_dir_all(
        decided_dir.to_str().ok_or_else(|| AppError::InvalidPath("Invalid decided dir".into()))?,
    )?;
    workspace.create_dir_all(
        requirements_dir
            .to_str()
            .ok_or_else(|| AppError::InvalidPath("Invalid requirements dir".into()))?,
    )?;

    // Create two mock requirements: one for planner, one for implementer
    let label = config.issue_labels.first().cloned().ok_or_else(|| {
        AppError::InvalidConfig("No issue labels available for mock decider".to_string())
    })?;
    let mock_issue_template = MOCK_ASSETS
        .get_file("decider_requirement.yml")
        .ok_or_else(|| {
            AppError::InternalError("Mock asset missing: decider_requirement.yml".to_string())
        })?
        .contents_utf8()
        .ok_or_else(|| {
            AppError::InternalError("Invalid UTF-8 in decider_requirement.yml".to_string())
        })?;

    // Move any mock pending events to decided first
    let mut moved_src_files: Vec<PathBuf> = Vec::new();
    for path in list_mock_tagged_files(workspace, &pending_dir, &config.mock_tag)? {
        let source = path
            .to_str()
            .ok_or_else(|| AppError::InvalidPath("Invalid pending event path".into()))?;
        let content = workspace.read_file(source)?;
        let dest = decided_dir.join(path.file_name().ok_or_else(|| {
            AppError::InvalidPath(format!("Pending event missing filename: {}", path.display()))
        })?);
        workspace.write_file(
            dest.to_str()
                .ok_or_else(|| AppError::InvalidPath("Invalid decided event path".into()))?,
            &content,
        )?;
        workspace.remove_file(source)?;
        moved_src_files.push(path);
    }

    let decided_mock_files = list_mock_tagged_files(workspace, &decided_dir, &config.mock_tag)?;
    let source_event_ids: Vec<String> = decided_mock_files
        .iter()
        .filter_map(|path| mock_event_id_from_path(path, &config.mock_tag))
        .collect();

    if source_event_ids.len() < 2 {
        return Err(AppError::InvalidConfig(format!(
            "Mock decider requires at least 2 decided events for tag '{}', found {}",
            config.mock_tag,
            source_event_ids.len()
        )));
    }

    let planner_source_event_ids = vec![source_event_ids[0].clone()];
    let impl_source_event_ids: Vec<String> = source_event_ids[1..].to_vec();

    // Requirement 1: requires deep analysis (for planner)
    let planner_issue_id = generate_mock_id();
    let planner_issue_file = requirements_dir.join(format!("planner-{}.yml", config.mock_tag));

    let mut planner_issue_yaml: serde_yaml::Value = serde_yaml::from_str(mock_issue_template)
        .map_err(|e| {
            AppError::InternalError(format!("Failed to parse mock issue template: {}", e))
        })?;

    if let Some(mapping) = planner_issue_yaml.as_mapping_mut() {
        mapping.insert("id".into(), planner_issue_id.clone().into());
        mapping.insert("label".into(), label.clone().into());
        mapping.insert(
            "summary".into(),
            format!(
                "This is a mock requirement created by jlo --mock for workflow-scaffold validation. Mock tag: {}",
                config.mock_tag
            )
            .into(),
        );
        let src_events = mapping
            .entry("source_events".into())
            .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));
        if let Some(seq) = src_events.as_sequence_mut() {
            seq.clear();
            for event_id in &planner_source_event_ids {
                seq.push(event_id.clone().into());
            }
        }

        mapping.insert("title".into(), "Mock requirement requiring deep analysis".into());
        mapping.insert("priority".into(), "high".into());
        mapping.insert("requires_deep_analysis".into(), true.into());
        mapping.insert(
            "deep_analysis_reason".into(),
            "Mock requirement requires architectural analysis-for-workflow-validation".into(),
        );
    }

    workspace.write_file(
        planner_issue_file.to_str().ok_or_else(|| {
            AppError::InvalidPath(format!(
                "Invalid planner requirement path: {}",
                planner_issue_file.display()
            ))
        })?,
        &serde_yaml::to_string(&planner_issue_yaml).map_err(|err| {
            AppError::InternalError(format!(
                "Failed to serialize planner requirement YAML: {}",
                err
            ))
        })?,
    )?;

    // Requirement 2: ready for implementer
    let impl_issue_id = generate_mock_id();
    let impl_issue_file = requirements_dir.join(format!("impl-{}.yml", config.mock_tag));

    let mut impl_issue_yaml: serde_yaml::Value = serde_yaml::from_str(mock_issue_template)
        .map_err(|e| {
            AppError::InternalError(format!("Failed to parse mock issue template: {}", e))
        })?;

    if let Some(mapping) = impl_issue_yaml.as_mapping_mut() {
        mapping.insert("id".into(), impl_issue_id.clone().into());
        mapping.insert("label".into(), label.clone().into());
        mapping.insert(
            "summary".into(),
            format!(
                "This is a mock requirement created by jlo --mock for workflow-scaffold validation. Mock tag: {}",
                config.mock_tag
            )
            .into(),
        );
        let src_events = mapping
            .entry("source_events".into())
            .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));
        if let Some(seq) = src_events.as_sequence_mut() {
            seq.clear();
            for event_id in &impl_source_event_ids {
                seq.push(event_id.clone().into());
            }
        }

        mapping.insert("title".into(), "Mock requirement ready for implementation".into());
        mapping.insert("requires_deep_analysis".into(), false.into());
    }

    workspace.write_file(
        impl_issue_file.to_str().ok_or_else(|| {
            AppError::InvalidPath(format!(
                "Invalid implementer requirement path: {}",
                impl_issue_file.display()
            ))
        })?,
        &serde_yaml::to_string(&impl_issue_yaml).map_err(|err| {
            AppError::InternalError(format!(
                "Failed to serialize implementer requirement YAML: {}",
                err
            ))
        })?,
    )?;

    // Ensure all tag-matched decided events have issue_id.
    let planner_event_set: HashSet<&str> =
        planner_source_event_ids.iter().map(|event_id| event_id.as_str()).collect();
    for decided_file in &decided_mock_files {
        if let Some(event_id) = mock_event_id_from_path(decided_file, &config.mock_tag) {
            let assigned_issue_id = if planner_event_set.contains(event_id.as_str()) {
                &planner_issue_id
            } else {
                &impl_issue_id
            };

            let decided_file_str = match decided_file.to_str() {
                Some(path) => path,
                None => {
                    println!(
                        "::warning::Invalid decided event file path (non UTF-8): {}",
                        decided_file.display()
                    );
                    continue;
                }
            };

            let content = match workspace.read_file(decided_file_str) {
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

            if let Err(err) = workspace.write_file(decided_file_str, &updated_content) {
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
    service.commit_and_push(
        &format!("[{}] decider: mock requirements", config.mock_tag),
        &files,
        &branch_name,
    )?;

    // Create PR
    let pr = service.create_pr(
        &branch_name,
        &config.jules_branch,
        &format!("[{}] Decider triage", config.mock_tag),
        &format!("Mock decider run for workflow validation.\n\nMock tag: `{}`\n\nCreated requirements:\n- `{}` (requires analysis)\n- `{}` (ready for impl)",
            config.mock_tag, planner_issue_id, impl_issue_id),
    )?;

    println!("Mock decider: created PR #{} ({})", pr.number, pr.url);

    let output = MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    };

    service.finish(&output)?;

    Ok(output)
}

pub fn execute_implementer_mock<G, H, W>(
    jules_path: &Path,
    requirement_path: Option<&Path>,
    base_branch_override: Option<&str>,
    config: &MockConfig,
    git: &G,
    github: &H,
    workspace: &W,
) -> Result<MockOutput, AppError>
where
    G: GitPort + ?Sized,
    H: GitHubPort + ?Sized,
    W: WorkspaceStore,
{
    let service = MockExecutionService::new(jules_path, config, git, github, workspace);

    let original_branch = git.get_current_branch()?;

    let requirement_path = requirement_path.ok_or_else(|| {
        AppError::MissingArgument("Requirement path is required for implementer".to_string())
    })?;

    // Parse requirement to get label and id
    let requirement_path_str = requirement_path
        .to_str()
        .ok_or_else(|| AppError::InvalidPath("Invalid requirement path".to_string()))?;

    let requirement_content = workspace.read_file(requirement_path_str)?;
    let (label, issue_id) = parse_requirement_for_branch(&requirement_content, requirement_path)?;
    if !config.issue_labels.contains(&label) {
        return Err(AppError::InvalidConfig(format!(
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
    let base_branch = base_branch_override.unwrap_or(&config.default_branch);
    service.fetch_and_checkout_base(base_branch)?;
    service.checkout_new_branch(&branch_name)?;

    // Create minimal mock file to have a commit
    let mock_file_path = format!(".{}", config.mock_tag);
    let mock_content = format!(
        "# Mock implementation marker\n# Mock tag: {}\n# Issue: {}\n# Created: {}\n",
        config.mock_tag,
        issue_id,
        chrono::Utc::now().to_rfc3339()
    );

    workspace.write_file(&mock_file_path, &mock_content)?;

    // Commit and push
    let mock_path = Path::new(&mock_file_path);
    let files: Vec<&Path> = vec![mock_path];
    service.commit_and_push(
        &format!("[{}] implementer: mock implementation", config.mock_tag),
        &files,
        &branch_name,
    )?;

    // Create PR targeting default branch (NOT jules)
    let pr = service.create_pr(
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

    // Restore original branch so post-run cleanup (requirement + source events) runs on
    // the exchange-bearing branch instead of the implementer branch.
    let restore_branch = if original_branch.trim().is_empty() {
        config.jules_branch.as_str()
    } else {
        &original_branch
    };
    service.git.checkout_branch(restore_branch, false)?;

    let output = MockOutput {
        mock_branch: branch_name,
        mock_pr_number: pr.number,
        mock_pr_url: pr.url,
        mock_tag: config.mock_tag.clone(),
    };

    service.finish(&output)?;

    Ok(output)
}

fn parse_requirement_for_branch(content: &str, path: &Path) -> Result<(String, String), AppError> {
    #[derive(Deserialize)]
    struct RequirementMeta {
        label: Option<String>,
        id: Option<String>,
    }

    let parsed: RequirementMeta = serde_yaml::from_str(content).map_err(|err| {
        AppError::InvalidConfig(format!(
            "Requirement file must be valid YAML ({}): {}",
            path.display(),
            err
        ))
    })?;

    let label = parsed.label.filter(|value| !value.trim().is_empty()).ok_or_else(|| {
        AppError::InvalidConfig(format!("Requirement file missing label field: {}", path.display()))
    })?;
    if !crate::domain::identifiers::validation::validate_safe_path_component(&label) {
        return Err(AppError::InvalidConfig(format!(
            "Requirement label '{}' is not a safe path component: {}",
            label,
            path.display()
        )));
    }

    let id = parsed.id.filter(|value| !value.trim().is_empty()).ok_or_else(|| {
        AppError::InvalidConfig(format!("Requirement file missing id field: {}", path.display()))
    })?;

    if id.len() != 6 || !id.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit()) {
        return Err(AppError::InvalidConfig(format!(
            "Issue id must be 6 lowercase alphanumeric chars: {}",
            path.display()
        )));
    }

    Ok((label, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockWorkspaceStore;
    use std::path::Path;

    #[test]
    fn test_generate_mock_id() {
        let id1 = generate_mock_id();
        let id2 = generate_mock_id();
        assert_eq!(id1.len(), 6);
        assert_eq!(id2.len(), 6);
    }

    #[test]
    fn test_mock_event_id_from_path() {
        let mock_tag = "mock-run-123";
        let valid_path = std::path::Path::new("mock-run-123-a1b2c3.yml");
        let invalid_path = std::path::Path::new("mock-other-tag-a1b2c3.yml");

        assert_eq!(mock_event_id_from_path(valid_path, mock_tag), Some("a1b2c3".to_string()));
        assert_eq!(mock_event_id_from_path(invalid_path, mock_tag), None);
    }

    #[test]
    fn list_mock_tagged_files_returns_only_tagged_sorted_yml_files() {
        let workspace = MockWorkspaceStore::new().with_exists(true);
        let decided_dir = Path::new("decided");

        workspace.write_file("decided/mock-run-123-bbbbbb.yml", "id: bbbbbb\n").unwrap();
        workspace.write_file("decided/mock-run-123-aaaaaa.yml", "id: aaaaaa\n").unwrap();
        workspace.write_file("decided/mock-other-run-cccccc.yml", "id: cccccc\n").unwrap();
        workspace.write_file("decided/notes.txt", "ignored\n").unwrap();

        let files = list_mock_tagged_files(&workspace, decided_dir, "mock-run-123").expect("list");

        assert_eq!(files.len(), 2);
        assert!(files[0].to_string_lossy().ends_with("mock-run-123-aaaaaa.yml"));
        assert!(files[1].to_string_lossy().ends_with("mock-run-123-bbbbbb.yml"));
    }

    #[test]
    fn output_value_validation_rejects_multiline_values() {
        assert!(ensure_single_line_output_value("mock_tag", "mock-run\ninjected").is_err());
        assert!(ensure_single_line_output_value("mock_tag", "mock-run\rinjected").is_err());
    }

    #[test]
    fn output_value_validation_accepts_single_line_values() {
        assert!(ensure_single_line_output_value("mock_tag", "mock-run-123").is_ok());
    }
}
