use std::path::{Path, PathBuf};

use crate::domain::identities::validation::validate_safe_path_component;
use crate::domain::{AppError, IoErrorKind, WorkstreamSchedule};
use crate::ports::WorkspaceStore;

pub fn load_schedule(
    store: &impl WorkspaceStore,
    workstream: &str,
) -> Result<WorkstreamSchedule, AppError> {
    // Validate workstream name to prevent path traversal
    if !validate_safe_path_component(workstream) {
        return Err(AppError::Validation(format!(
            "Invalid workstream name '{}': must be alphanumeric with hyphens or underscores only",
            workstream
        )));
    }

    let path = store.jules_path().join("workstreams").join(workstream).join("scheduled.toml");

    let content = store.read_file(path.to_str().unwrap()).map_err(|err| {
        if matches!(err, AppError::Io { kind: IoErrorKind::NotFound, .. }) {
            AppError::ScheduleConfigMissing(path.display().to_string())
        } else {
            err
        }
    })?;
    Ok(WorkstreamSchedule::parse_toml(&content)?)
}

pub fn list_subdirectories(
    store: &impl WorkspaceStore,
    dir: &Path,
) -> Result<Vec<PathBuf>, AppError> {
    let entries = store.list_dir(dir.to_str().unwrap())?;
    let mut subdirs = Vec::new();
    for entry in entries {
        if store.is_dir(entry.to_str().unwrap()) {
            subdirs.push(entry);
        }
    }
    subdirs.sort();
    Ok(subdirs)
}
