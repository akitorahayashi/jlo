use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::git::GitCommandAdapter;
use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::app::commands::workflow::push::{
    PushWorkerBranchOptions, execute as push_worker_branch,
};
use crate::domain::AppError;
use crate::domain::PromptAssetLoader;
use crate::ports::{Git, JloStore, JulesStore, RepositoryFilesystem};

use crate::app::commands::workflow::exchange::inspect::inspect_at;

#[derive(Debug, Clone)]
pub struct ExchangeCleanRequirementOptions {
    pub requirement_file: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeCleanRequirementOutput {
    pub schema_version: u32,
    pub deleted_paths: Vec<String>,
    pub committed: bool,
    pub commit_sha: String,
    pub pushed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ExchangeCleanRequirementApplyOutput {
    pub schema_version: u32,
    pub deleted_paths: Vec<String>,
    pub requirement_id: String,
}

pub fn execute(
    options: ExchangeCleanRequirementOptions,
) -> Result<ExchangeCleanRequirementOutput, AppError> {
    let repository = LocalRepositoryAdapter::current()?;
    let root = repository_root(&repository)?;
    let git = GitCommandAdapter::new(root);
    execute_with_adapters(options, &repository, &git)
}

pub fn execute_with_adapters<
    G: Git,
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
>(
    options: ExchangeCleanRequirementOptions,
    repository: &W,
    git: &G,
) -> Result<ExchangeCleanRequirementOutput, AppError> {
    let applied = apply_with_adapters(options, repository, git)?;
    let push_output = push_worker_branch(PushWorkerBranchOptions {
        change_token: format!("requirement-cleanup-{}", applied.requirement_id),
        commit_message: format!("jules: clean requirement {}", applied.requirement_id),
        pr_title: format!("chore: clean requirement {}", applied.requirement_id),
        pr_body: format!(
            "Automated cleanup for processed requirement `{}`.\n\n- remove requirement artifact\n- remove source event artifacts",
            applied.requirement_id
        ),
    })?;

    let commit_sha = push_output.head_sha.unwrap_or_default();
    let committed = push_output.applied;
    let pushed = push_output.applied;

    Ok(ExchangeCleanRequirementOutput {
        schema_version: 1,
        deleted_paths: applied.deleted_paths,
        committed,
        commit_sha,
        pushed,
        pr_number: push_output.pr_number,
    })
}

pub fn apply_with_adapters<
    G: Git,
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader,
>(
    options: ExchangeCleanRequirementOptions,
    repository: &W,
    git: &G,
) -> Result<ExchangeCleanRequirementApplyOutput, AppError> {
    if !repository.jules_exists() {
        return Err(AppError::JulesNotFound);
    }

    let jules_path = repository.jules_path();
    let canonical_jules = repository
        .canonicalize(path_to_str(&jules_path, "Invalid .jules path")?)
        .map_err(|e| AppError::InternalError(format!("Failed to resolve .jules path: {}", e)))?;

    let canonical_requirement =
        repository.canonicalize(&options.requirement_file).map_err(|_| {
            AppError::Validation(format!(
                "Requirement file does not exist: {}",
                options.requirement_file
            ))
        })?;

    if !canonical_requirement.starts_with(&canonical_jules) {
        return Err(AppError::Validation(format!(
            "Requirement file must be within .jules/ directory: {}",
            options.requirement_file
        )));
    }

    let requirement_rel =
        resolve_requirement_path(&canonical_jules, &canonical_requirement, repository)?;

    let inspect_output = inspect_at(repository)?;

    let requirement_item = inspect_output
        .requirements
        .items
        .iter()
        .find(|item| item.path == requirement_rel)
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Requirement file not found in inspection output: {}",
                requirement_rel
            ))
        })?;

    let mut event_map: HashMap<&str, &str> = HashMap::new();
    for event in &inspect_output.events.items {
        event_map.insert(event.id.as_str(), event.path.as_str());
    }

    let mut deleted_paths = HashSet::new();
    for event_id in &requirement_item.source_events {
        let event_path = event_map.get(event_id.as_str()).ok_or_else(|| {
            AppError::Validation(format!(
                "Source event '{}' not found in inspection output",
                event_id
            ))
        })?;
        deleted_paths.insert(event_path.to_string());
    }

    deleted_paths.insert(requirement_rel.clone());

    let mut deleted_paths: Vec<String> = deleted_paths.into_iter().collect();
    deleted_paths.sort();

    if deleted_paths.is_empty() {
        return Err(AppError::Validation(
            "No files resolved for cleanup; aborting to avoid empty commit".to_string(),
        ));
    }

    for path in &deleted_paths {
        git.run_command(&["rm", "--", path], None)?;
    }

    Ok(ExchangeCleanRequirementApplyOutput {
        schema_version: 1,
        deleted_paths,
        requirement_id: requirement_item.id.clone(),
    })
}

fn resolve_requirement_path<
    W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader + ?Sized,
>(
    canonical_jules: &Path,
    canonical_requirement: &Path,
    repository: &W,
) -> Result<String, AppError> {
    let rel_to_jules = canonical_requirement
        .strip_prefix(canonical_jules)
        .map_err(|_| AppError::Validation("Requirement file is not under .jules/".to_string()))?;

    let parts: Vec<String> =
        rel_to_jules.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();

    if parts.len() < 3 || parts[0] != "exchange" || parts[1] != "requirements" {
        return Err(AppError::Validation(format!(
            "Requirement file must be under .jules/exchange/requirements/: {}",
            canonical_requirement.display()
        )));
    }

    let root = repository_root(repository)?;
    let requirement_rel = to_repo_relative(&root, canonical_requirement);

    Ok(requirement_rel)
}

fn repository_root<W: RepositoryFilesystem + JloStore + JulesStore + PromptAssetLoader + ?Sized>(
    repository: &W,
) -> Result<PathBuf, AppError> {
    let jules_path = repository.jules_path();
    let root = jules_path.parent().ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid .jules path (missing parent): {}",
            jules_path.display()
        ))
    })?;
    let root = root.to_path_buf();
    let root_str = path_to_str(&root, "Repository root contains invalid unicode")?;
    repository
        .canonicalize(root_str)
        .map_err(|e| AppError::InternalError(format!("Failed to resolve repository root: {}", e)))
}

fn to_repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string()
}

fn path_to_str<'a>(path: &'a Path, err_prefix: &str) -> Result<&'a str, AppError> {
    path.to_str().ok_or_else(|| AppError::Validation(format!("{}: {}", err_prefix, path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::git::GitCommandAdapter;
    use crate::adapters::local_repository::LocalRepositoryAdapter;
    use serial_test::serial;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn clean_requirement_apply_deletes_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let repo_dir = root.join("repo");
        let remote_dir = root.join("remote.git");
        fs::create_dir_all(&repo_dir).unwrap();

        Command::new("git").args(["init"]).current_dir(&repo_dir).output().unwrap();
        Command::new("git")
            .args(["checkout", "-b", "jules"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();

        Command::new("git")
            .args(["init", "--bare", remote_dir.to_str().unwrap()])
            .output()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", remote_dir.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output()
            .unwrap();

        let jules_path = repo_dir.join(".jules");
        let jlo_path = repo_dir.join(".jlo");
        let exchange_dir = jules_path.join("exchange");
        fs::create_dir_all(exchange_dir.join("events/pending")).unwrap();
        fs::create_dir_all(exchange_dir.join("requirements")).unwrap();

        fs::write(exchange_dir.join("events/pending/event1.yml"), "id: abc123\n").unwrap();
        fs::write(exchange_dir.join("events/pending/event2.yml"), "id: def456\n").unwrap();
        fs::write(
            exchange_dir.join("requirements/issue.yml"),
            r#"
id: abc123
label: bugs
source_events:
  - abc123
  - def456
requires_deep_analysis: false
"#,
        )
        .unwrap();

        fs::create_dir_all(&jlo_path).unwrap();
        fs::write(
            jlo_path.join("config.toml"),
            r#"
[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[observers]
roles = [
    { name = "taxonomy", enabled = true },
]
"#,
        )
        .unwrap();

        Command::new("git").args(["add", ".jules"]).current_dir(&repo_dir).output().unwrap();
        Command::new("git").args(["add", ".jlo"]).current_dir(&repo_dir).output().unwrap();
        Command::new("git").args(["commit", "-m", "seed"]).current_dir(&repo_dir).output().unwrap();

        std::env::set_current_dir(&repo_dir).unwrap();

        let repository = LocalRepositoryAdapter::new(repo_dir.clone());
        let git = GitCommandAdapter::new(repo_dir.clone());
        let output = apply_with_adapters(
            ExchangeCleanRequirementOptions {
                requirement_file: ".jules/exchange/requirements/issue.yml".to_string(),
            },
            &repository,
            &git,
        )
        .unwrap();

        assert_eq!(output.schema_version, 1);
        assert!(output.deleted_paths.iter().any(|p| p.contains("event1.yml")));
        assert!(output.deleted_paths.iter().any(|p| p.contains("event2.yml")));
        assert!(output.deleted_paths.iter().any(|p| p.contains("issue.yml")));

        assert!(!repo_dir.join(".jules/exchange/events/pending/event1.yml").exists());
        assert!(!repo_dir.join(".jules/exchange/events/pending/event2.yml").exists());
        assert!(!repo_dir.join(".jules/exchange/requirements/issue.yml").exists());

        let status = Command::new("git")
            .args(["status", "--porcelain", "--", ".jules"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        assert!(
            !String::from_utf8_lossy(&status.stdout).trim().is_empty(),
            "cleanup apply should stage/track .jules changes"
        );
    }
}
