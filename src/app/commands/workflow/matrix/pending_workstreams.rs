//! Matrix pending-workstreams command implementation.
//!
//! Exports workstreams with pending events as a GitHub Actions matrix.

use serde::{Deserialize, Serialize};
use std::fs;

use crate::adapters::workspace_filesystem::FilesystemWorkspaceStore;
use crate::domain::AppError;
use crate::ports::WorkspaceStore;

/// Options for matrix pending-workstreams command.
#[derive(Debug, Clone)]
pub struct MatrixPendingWorkstreamsOptions {
    /// Workstreams JSON from `matrix workstreams` output.
    pub workstreams_json: WorkstreamsMatrix,
    /// Mock mode - treat all workstreams as having pending events.
    pub mock: bool,
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

/// Output of matrix pending-workstreams command.
#[derive(Debug, Clone, Serialize)]
pub struct MatrixPendingWorkstreamsOutput {
    /// Schema version for output format stability.
    pub schema_version: u32,
    /// GitHub Actions matrix object.
    pub matrix: PendingWorkstreamsMatrix,
    /// Number of workstreams with pending events.
    pub count: usize,
    /// Whether any workstreams have pending events.
    pub has_pending: bool,
}

/// GitHub Actions matrix structure for pending workstreams.
#[derive(Debug, Clone, Serialize)]
pub struct PendingWorkstreamsMatrix {
    /// Matrix include entries.
    pub include: Vec<PendingWorkstreamEntry>,
}

/// Single pending workstream matrix entry.
#[derive(Debug, Clone, Serialize)]
pub struct PendingWorkstreamEntry {
    /// Workstream name.
    pub workstream: String,
}

/// Execute matrix pending-workstreams command.
pub fn execute(
    options: MatrixPendingWorkstreamsOptions,
) -> Result<MatrixPendingWorkstreamsOutput, AppError> {
    let workspace = FilesystemWorkspaceStore::current()?;

    if !workspace.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let mut include = Vec::new();

    if options.mock {
        // In mock mode, treat all input workstreams as having pending events
        for ws_entry in &options.workstreams_json.include {
            include.push(PendingWorkstreamEntry { workstream: ws_entry.workstream.clone() });
        }
    } else {
        let jules_path = workspace.jules_path();

        for ws_entry in &options.workstreams_json.include {
            let pending_dir = jules_path
                .join("workstreams")
                .join(&ws_entry.workstream)
                .join("exchange/events/pending");

            if pending_dir.exists() && has_yml_files(&pending_dir)? {
                include.push(PendingWorkstreamEntry { workstream: ws_entry.workstream.clone() });
            }
        }
    }

    // Ensure deterministic ordering
    include.sort_by(|a, b| a.workstream.cmp(&b.workstream));

    let count = include.len();
    let has_pending = !include.is_empty();

    Ok(MatrixPendingWorkstreamsOutput {
        schema_version: 1,
        matrix: PendingWorkstreamsMatrix { include },
        count,
        has_pending,
    })
}

/// Check if a directory contains any .yml files.
fn has_yml_files(dir: &std::path::Path) -> Result<bool, AppError> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "yml").unwrap_or(false) {
            return Ok(true);
        }
    }
    Ok(false)
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

    fn create_pending_event(root: &std::path::Path, ws: &str, event_name: &str) {
        let pending_dir = root.join(format!(".jules/workstreams/{}/exchange/events/pending", ws));
        fs::create_dir_all(&pending_dir).unwrap();
        fs::write(pending_dir.join(format!("{}.yml", event_name)), "id: abc123\n").unwrap();
    }

    fn create_empty_pending_dir(root: &std::path::Path, ws: &str) {
        let pending_dir = root.join(format!(".jules/workstreams/{}/exchange/events/pending", ws));
        fs::create_dir_all(&pending_dir).unwrap();
    }

    #[test]
    fn returns_workstreams_with_pending_events() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        // alpha has pending events
        create_pending_event(root, "alpha", "event1");

        // beta has empty pending dir
        create_empty_pending_dir(root, "beta");

        // gamma has pending events
        create_pending_event(root, "gamma", "event2");

        std::env::set_current_dir(root).unwrap();

        let workstreams_json = WorkstreamsMatrix {
            include: vec![
                WorkstreamEntry { workstream: "alpha".into() },
                WorkstreamEntry { workstream: "beta".into() },
                WorkstreamEntry { workstream: "gamma".into() },
            ],
        };

        let output =
            execute(MatrixPendingWorkstreamsOptions { workstreams_json, mock: false }).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.count, 2);
        assert!(output.has_pending);

        let names: Vec<&str> =
            output.matrix.include.iter().map(|e| e.workstream.as_str()).collect();
        assert_eq!(names, vec!["alpha", "gamma"]);
    }

    #[test]
    fn returns_empty_when_no_pending_events() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        setup_workspace(root);

        create_empty_pending_dir(root, "alpha");

        std::env::set_current_dir(root).unwrap();

        let workstreams_json =
            WorkstreamsMatrix { include: vec![WorkstreamEntry { workstream: "alpha".into() }] };

        let output =
            execute(MatrixPendingWorkstreamsOptions { workstreams_json, mock: false }).unwrap();

        assert_eq!(output.schema_version, 1);
        assert_eq!(output.count, 0);
        assert!(!output.has_pending);
        assert!(output.matrix.include.is_empty());
    }
}
