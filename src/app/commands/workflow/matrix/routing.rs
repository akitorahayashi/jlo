//! Matrix routing command implementation.
//!
//! Exports planner/implementer issue matrices from workstream inspection and routing labels.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

<<<<<<< HEAD
use crate::adapters::issue_filesystem::read_issue_header;
=======
>>>>>>> 27748b9 (Refactor codebase structure to separate layers and enforce DI)
use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Options for matrix routing command.
#[derive(Debug, Clone)]
pub struct MatrixRoutingOptions {
    /// Workstreams JSON from `matrix workstreams` output.
    pub workstreams_json: WorkstreamsMatrix,
    /// Routing labels as CSV (e.g., "bugs,feats,refacts,tests,docs").
    pub routing_labels: String,
    /// Optional workspace root path.
    pub workspace_root: Option<PathBuf>,
}

/// Input workstreams matrix (from matrix workstreams output).
#[derive(Debug, Clone, Deserialize)]
pub struct WorkstreamsMatrix {
    /// Matrix include entries.
    pub include: Vec<WorkstreamEntry>,
}

/// Single workstream entry from input matrix.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkstreamEntry {
    /// Workstream name.
    pub workstream: String,
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
    /// Workstream name.
    pub workstream: String,
    /// Issue file path (relative to repo root).
    pub issue: String,
}

/// Execute matrix routing command.
pub fn execute(options: MatrixRoutingOptions) -> Result<MatrixRoutingOutput, AppError> {
    let workspace = match options.workspace_root {
        Some(root) => FilesystemWorkspaceStore::new(root),
        None => FilesystemWorkspaceStore::current()?,
    };

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = workspace.jules_path();
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

    for ws_entry in &options.workstreams_json.include {
        if ws_entry.workstream.contains("..")
            || ws_entry.workstream.contains('/')
            || ws_entry.workstream.contains('\\')
        {
            return Err(AppError::Validation(format!(
                "Invalid workstream name '{}': must not contain path separators or '..'",
                ws_entry.workstream
            )));
        }

        let issues_dir =
            jules_path.join("workstreams").join(&ws_entry.workstream).join("exchange/issues");

        if !issues_dir.exists() {
            continue;
        }

        // Only scan directories matching routing labels
        for label in &labels {
            let label_dir = issues_dir.join(label);
            if !label_dir.exists() {
                continue;
            }

            let files = list_yml_files(&label_dir)?;
            for file_path in files {
                let header = read_issue_header(&file_path)?;
                let requires_deep = header.requires_deep_analysis;
                let rel_path = to_repo_relative(root, &file_path);

                if requires_deep {
                    planner_issues.push(IssueMatrixEntry {
                        workstream: ws_entry.workstream.clone(),
                        issue: rel_path,
                    });
                } else {
                    implementer_issues.push(IssueMatrixEntry {
                        workstream: ws_entry.workstream.clone(),
                        issue: rel_path,
                    });
                }
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

fn list_yml_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, AppError> {
    let mut files: Vec<std::path::PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "yml").unwrap_or(false))
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
    use std::fs;
    use tempfile::tempdir;

    fn setup_workspace(root: &std::path::Path) {
        fs::create_dir_all(root.join(".jules")).unwrap();
        fs::write(root.join(".jules/version"), env!("CARGO_PKG_VERSION")).unwrap();
    }

    fn create_issue(root: &std::path::Path, ws: &str, label: &str, name: &str, deep: bool) {
        let issues_dir = root.join(format!(".jules/workstreams/{}/exchange/issues/{}", ws, label));
        fs::create_dir_all(&issues_dir).unwrap();
        let content =
            format!("id: {}\nrequires_deep_analysis: {}\nsource_events:\n  - event1\n", name, deep);
        fs::write(issues_dir.join(format!("{}.yml", name)), content).unwrap();
    }

    #[test]
    fn routes_issues_by_deep_analysis() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        // Create issues with different requires_deep_analysis values
        create_issue(root, "alpha", "bugs", "abc123", true); // planner
        create_issue(root, "alpha", "bugs", "def456", false); // implementer
        create_issue(root, "alpha", "feats", "ghi789", true); // planner
        create_issue(root, "alpha", "docs", "jkl012", false); // implementer (but not in routing)

        let workstreams_json =
            WorkstreamsMatrix { include: vec![WorkstreamEntry { workstream: "alpha".into() }] };

        let output = execute(MatrixRoutingOptions {
            workstreams_json,
            routing_labels: "bugs,feats".into(),
            workspace_root: Some(root.to_path_buf()),
        })
        .unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.planner_count, 2);
        assert!(output.has_planners);
        assert_eq!(output.implementer_count, 1);
        assert!(output.has_implementers);

        // Check planner issues (should be sorted)
        let planner_paths: Vec<&str> =
            output.planner_matrix.include.iter().map(|e| e.issue.as_str()).collect();
        let planner_workstreams: Vec<&str> =
            output.planner_matrix.include.iter().map(|e| e.workstream.as_str()).collect();
        assert!(planner_paths[0].contains("bugs/abc123.yml"));
        assert!(planner_paths[1].contains("feats/ghi789.yml"));
        assert_eq!(planner_workstreams, vec!["alpha", "alpha"]);

        // Check implementer issues
        let impl_paths: Vec<&str> =
            output.implementer_matrix.include.iter().map(|e| e.issue.as_str()).collect();
        let impl_workstreams: Vec<&str> =
            output.implementer_matrix.include.iter().map(|e| e.workstream.as_str()).collect();
        assert!(impl_paths[0].contains("bugs/def456.yml"));
        assert_eq!(impl_workstreams, vec!["alpha"]);

        // docs label not in routing_labels, so jkl012 should not appear
        assert!(
            !planner_paths.iter().any(|p| p.contains("docs")),
            "docs issues should not be in planner"
        );
        assert!(
            !impl_paths.iter().any(|p| p.contains("docs")),
            "docs issues should not be in implementer"
        );
    }

    #[test]
    fn rejects_empty_routing_labels() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        let result = execute(MatrixRoutingOptions {
            workstreams_json: WorkstreamsMatrix { include: vec![] },
            routing_labels: "".into(),
            workspace_root: Some(root.to_path_buf()),
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("routing_labels must not be empty"));
    }
}
