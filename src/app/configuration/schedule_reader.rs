//! Schedule loading from repository.

use std::path::{Path, PathBuf};

use crate::domain::{AppError, IoErrorKind, Schedule};
use crate::ports::{JloStore, RepositoryFilesystem};

/// Load the root schedule from `.jlo/scheduled.toml`.
pub fn load_schedule(store: &(impl RepositoryFilesystem + JloStore)) -> Result<Schedule, AppError> {
    let path = store.jlo_path().join("scheduled.toml");
    let path_str = path.to_string_lossy();

    let content = store.read_file(&path_str).map_err(|err| {
        if matches!(err, AppError::Io { kind: IoErrorKind::NotFound, .. }) {
            AppError::ScheduleConfigMissing(path.display().to_string())
        } else {
            err
        }
    })?;
    Ok(Schedule::parse_toml(&content)?)
}

/// List subdirectories of a directory via repository store.
pub fn list_subdirectories(
    store: &impl RepositoryFilesystem,
    dir: &Path,
) -> Result<Vec<PathBuf>, AppError> {
    let entries = store.list_dir(&dir.to_string_lossy())?;
    let mut subdirs: Vec<PathBuf> =
        entries.into_iter().filter(|entry| store.is_dir(&entry.to_string_lossy())).collect();
    subdirs.sort();
    Ok(subdirs)
}
