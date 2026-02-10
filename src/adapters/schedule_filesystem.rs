use std::path::{Path, PathBuf};

use crate::domain::{AppError, IoErrorKind, Schedule};
use crate::ports::WorkspaceStore;

/// Load the root schedule from `.jlo/scheduled.toml`.
pub fn load_schedule(store: &impl WorkspaceStore) -> Result<Schedule, AppError> {
    let path = store.jlo_path().join("scheduled.toml");
    let path_str = path.to_str().unwrap();

    let content = store.read_file(path_str).map_err(|err| {
        if matches!(err, AppError::Io { kind: IoErrorKind::NotFound, .. }) {
            AppError::ScheduleConfigMissing(path.display().to_string())
        } else {
            err
        }
    })?;
    Ok(Schedule::parse_toml(&content)?)
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
