//! Matrix routing command implementation.
//!
//! Exports planner/implementer issue matrices from workstream inspection and routing labels.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::domain::{AppError, IssueHeader};
use crate::ports::WorkspaceStore;

/// Options for matrix routing command.
#[derive(Debug, Clone)]
pub struct MatrixRoutingOptions {
    /// Workstreams JSON from `matrix workstreams` output.
    pub workstreams_json: WorkstreamsMatrix,
    /// Routing labels as CSV (e.g., "bugs,feats,refacts,tests,docs").
    pub routing_labels: String,
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

        let issues_dir_str = match issues_dir.to_str() {
            Some(s) => s,
            None => {
                return Err(AppError::Validation(format!(
                    "Invalid path: {}",
                    issues_dir.display()
                )));
            }
        };

        if !store.file_exists(issues_dir_str) {
            continue;
        }

        // Only scan directories matching routing labels
        for label in &labels {
            let label_dir = issues_dir.join(label);
            let label_dir_str = match label_dir.to_str() {
                Some(s) => s,
                None => {
                    return Err(AppError::Validation(format!(
                        "Invalid path: {}",
                        label_dir.display()
                    )));
                }
            };

            if !store.file_exists(label_dir_str) {
                continue;
            }

            let files = list_yml_files(store, &label_dir)?;
            for file_path in files {
                let requires_deep = IssueHeader::read(store, &file_path)?.requires_deep_analysis;
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

    fn create_issue(store: &MemoryWorkspaceStore, ws: &str, label: &str, name: &str, deep: bool) {
        let issues_dir = format!(".jules/workstreams/{}/exchange/issues/{}", ws, label);
        let content =
            format!("id: {}\nrequires_deep_analysis: {}\nsource_events:\n  - event1\n", name, deep);
        let path = format!("{}/{}.yml", issues_dir, name);
        store.write_file(&path, &content).unwrap();
    }

    #[test]
    fn routes_issues_by_deep_analysis() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        // Create issues with different requires_deep_analysis values
        create_issue(&store, "alpha", "bugs", "abc123", true); // planner
        create_issue(&store, "alpha", "bugs", "def456", false); // implementer
        create_issue(&store, "alpha", "feats", "ghi789", true); // planner
        create_issue(&store, "alpha", "docs", "jkl012", false); // implementer (but not in routing)

        let workstreams_json =
            WorkstreamsMatrix { include: vec![WorkstreamEntry { workstream: "alpha".into() }] };

        let output = execute(
            &store,
            MatrixRoutingOptions { workstreams_json, routing_labels: "bugs,feats".into() },
        )
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
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        let result = execute(
            &store,
            MatrixRoutingOptions {
                workstreams_json: WorkstreamsMatrix { include: vec![] },
                routing_labels: "".into(),
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("routing_labels must not be empty"));
    }
}
