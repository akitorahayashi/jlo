//! Matrix routing command implementation.
//!
//! Exports planner/implementer issue matrices from flat exchange and routing labels.

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::domain::{AppError, RequirementHeader};
use crate::ports::WorkspaceStore;

/// Options for matrix routing command.
#[derive(Debug, Clone)]
pub struct MatrixRoutingOptions {
    /// Routing labels as CSV (e.g., "bugs,feats,refacts,tests,docs").
    pub routing_labels: String,
}

/// Output of matrix routing command.
#[derive(Debug, Clone, Serialize)]
pub struct MatrixRoutingOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// Planner matrix (issues requiring deep analysis).
    pub planner_matrix: IssueMatrix,
    /// Number of planner issues.
    pub planner_count: usize,
    /// Whether any planner issues exist.
    pub has_planners: bool,
    /// Implementer matrix (issues not requiring deep analysis).
    pub implementer_matrix: IssueMatrix,
    /// Number of implementer issues.
    pub implementer_count: usize,
    /// Whether any implementer issues exist.
    pub has_implementers: bool,
}

/// GitHub Actions matrix structure for issues.
#[derive(Debug, Clone, Serialize)]
pub struct IssueMatrix {
    /// Matrix include entries.
    pub include: Vec<IssueMatrixEntry>,
}

/// Single issue matrix entry.
#[derive(Debug, Clone, Serialize)]
pub struct IssueMatrixEntry {
    /// Issue file path (relative to repo root).
    pub issue: String,
}

/// Execute matrix routing command.
pub fn execute(
    store: &impl WorkspaceStore,
    options: MatrixRoutingOptions,
) -> Result<MatrixRoutingOutput, AppError> {
    if !store.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = store.jules_path();
    let root = jules_path.parent().unwrap_or(Path::new("."));

    // Parse routing labels
    let labels: Vec<&str> =
        options.routing_labels.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    if labels.is_empty() {
        return Err(AppError::Validation("routing_labels must not be empty".into()));
    }

    // Validate labels don't contain path traversal sequences
    for label in &labels {
        if label.contains("..") || label.contains('/') || label.contains('\\') {
            return Err(AppError::Validation(format!(
                "Invalid routing label '{}': must not contain path separators or '..'",
                label
            )));
        }
    }

    let mut planner_issues = Vec::new();
    let mut implementer_issues = Vec::new();

    let requirements_dir = jules_path.join("exchange/requirements");

    let requirements_dir_str = match requirements_dir.to_str() {
        Some(s) => s,
        None => {
            return Err(AppError::Validation(format!(
                "Invalid path: {}",
                requirements_dir.display()
            )));
        }
    };

    if store.file_exists(requirements_dir_str) {
        let files = list_yml_files(store, &requirements_dir)?;
        for file_path in files {
            let header = RequirementHeader::read(store, &file_path)?;
            let label_str = header.label.as_deref().unwrap_or("");

            // Only include requirements whose label is in routing_labels
            if !labels.contains(&label_str) {
                continue;
            }

            let rel_path = to_repo_relative(root, &file_path);

            if header.requires_deep_analysis {
                planner_issues.push(IssueMatrixEntry { issue: rel_path });
            } else {
                implementer_issues.push(IssueMatrixEntry { issue: rel_path });
            }
        }
    }

    // Ensure deterministic ordering
    planner_issues.sort_by(|a, b| a.issue.cmp(&b.issue));
    implementer_issues.sort_by(|a, b| a.issue.cmp(&b.issue));

    let planner_count = planner_issues.len();
    let implementer_count = implementer_issues.len();

    Ok(MatrixRoutingOutput {
        schema_version: 1,
        planner_matrix: IssueMatrix { include: planner_issues },
        planner_count,
        has_planners: planner_count > 0,
        implementer_matrix: IssueMatrix { include: implementer_issues },
        implementer_count,
        has_implementers: implementer_count > 0,
    })
}

fn list_yml_files(store: &impl WorkspaceStore, dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let dir_str = dir
        .to_str()
        .ok_or_else(|| AppError::Validation(format!("Invalid path: {}", dir.display())))?;

    let entries = store.list_dir(dir_str)?;
    let mut files: Vec<PathBuf> = entries
        .into_iter()
        .filter(|path| {
            let is_yml = path.extension().map(|ext| ext == "yml").unwrap_or(false);
            if !is_yml {
                return false;
            }
            // Ensure it's not a directory
            match path.to_str() {
                Some(p) => !store.is_dir(p),
                None => false,
            }
        })
        .collect();
    files.sort();
    Ok(files)
}

fn to_repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory_workspace_store::MemoryWorkspaceStore;
    use crate::ports::WorkspaceStore;

    // These tests use MemoryWorkspaceStore and do not modify the process-wide current working directory,
    // ensuring safe parallel execution.
    fn setup_workspace(store: &MemoryWorkspaceStore) {
        store.write_version(env!("CARGO_PKG_VERSION")).unwrap();
    }

    fn create_issue(store: &MemoryWorkspaceStore, name: &str, label: &str, deep: bool) {
        let content = format!(
            "id: {}\nlabel: {}\nrequires_deep_analysis: {}\nsource_events:\n  - event1\n",
            name, label, deep
        );
        let path = format!(".jules/exchange/requirements/{}.yml", name);
        store.write_file(&path, &content).unwrap();
    }

    #[test]
    fn routes_issues_by_deep_analysis() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        create_issue(&store, "abc123", "bugs", true);
        create_issue(&store, "def456", "bugs", false);
        create_issue(&store, "ghi789", "feats", true);
        create_issue(&store, "jkl012", "docs", false);

        let output =
            execute(&store, MatrixRoutingOptions { routing_labels: "bugs,feats".into() }).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.planner_count, 2);
        assert!(output.has_planners);
        assert_eq!(output.implementer_count, 1);
        assert!(output.has_implementers);

        let planner_paths: Vec<&str> =
            output.planner_matrix.include.iter().map(|e| e.issue.as_str()).collect();
        assert!(planner_paths[0].contains("abc123.yml"));
        assert!(planner_paths[1].contains("ghi789.yml"));

        let impl_paths: Vec<&str> =
            output.implementer_matrix.include.iter().map(|e| e.issue.as_str()).collect();
        assert!(impl_paths[0].contains("def456.yml"));

        assert!(
            !planner_paths.iter().any(|p| p.contains("jkl012")),
            "docs issues should not be in planner"
        );
        assert!(
            !impl_paths.iter().any(|p| p.contains("jkl012")),
            "docs issues should not be in implementer"
        );
    }

    #[test]
    fn rejects_empty_routing_labels() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        let result = execute(&store, MatrixRoutingOptions { routing_labels: "".into() });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("routing_labels must not be empty"));
    }
}
