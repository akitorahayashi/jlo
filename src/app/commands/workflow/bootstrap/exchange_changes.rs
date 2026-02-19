//! Workflow bootstrap exchange-changes subcommand.
//!
//! Removes `.jules/exchange/changes.yml` so narrator always rebuilds a fresh
//! summary from recent history.

use serde::Serialize;

use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::domain::AppError;
use crate::ports::{JulesStore, RepositoryFilesystem};

/// Options for `workflow bootstrap exchange-changes`.
#[derive(Debug)]
pub struct WorkflowBootstrapExchangeChangesOptions {
    /// Root path of the repository.
    pub root: std::path::PathBuf,
}

/// Output of `workflow bootstrap exchange-changes`.
#[derive(Debug, Serialize)]
pub struct WorkflowBootstrapExchangeChangesOutput {
    pub schema_version: u32,
    pub removed: bool,
    pub path: String,
}

/// Execute `workflow bootstrap exchange-changes`.
pub fn execute(
    options: WorkflowBootstrapExchangeChangesOptions,
) -> Result<WorkflowBootstrapExchangeChangesOutput, AppError> {
    super::validate_control_plane_preconditions(options.root.as_path())?;

    let repository = LocalRepositoryAdapter::new(options.root);
    let changes_path = crate::domain::exchange::paths::exchange_changes(&repository.jules_path());
    let path = changes_path
        .to_str()
        .ok_or_else(|| {
            AppError::Validation("changes.yml path contains invalid unicode".to_string())
        })?
        .to_string();

    let removed = if repository.file_exists(&path) {
        repository.remove_file(&path)?;
        true
    } else {
        false
    };

    Ok(WorkflowBootstrapExchangeChangesOutput { schema_version: 1, removed, path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn seed_control_plane(root: &std::path::Path) {
        fs::create_dir_all(root.join(".jlo")).unwrap();
        fs::write(root.join(".jlo/.jlo-version"), format!("{}\n", env!("CARGO_PKG_VERSION")))
            .unwrap();
    }

    #[test]
    fn removes_existing_changes_file() {
        let temp = tempdir().unwrap();
        seed_control_plane(temp.path());
        fs::create_dir_all(temp.path().join(".jules/exchange")).unwrap();
        fs::write(temp.path().join(".jules/exchange/changes.yml"), "schema_version: 1\n").unwrap();

        let output =
            execute(WorkflowBootstrapExchangeChangesOptions { root: temp.path().to_path_buf() })
                .unwrap();

        assert!(output.removed);
        assert!(!temp.path().join(".jules/exchange/changes.yml").exists());
    }

    #[test]
    fn noops_when_changes_file_missing() {
        let temp = tempdir().unwrap();
        seed_control_plane(temp.path());
        fs::create_dir_all(temp.path().join(".jules/exchange")).unwrap();

        let output =
            execute(WorkflowBootstrapExchangeChangesOptions { root: temp.path().to_path_buf() })
                .unwrap();

        assert!(!output.removed);
    }
}
