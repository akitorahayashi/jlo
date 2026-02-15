//! Schedule loading from repository.

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
