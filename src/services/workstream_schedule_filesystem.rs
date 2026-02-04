use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{AppError, WorkstreamSchedule};

pub fn load_schedule(jules_path: &Path, workstream: &str) -> Result<WorkstreamSchedule, AppError> {
    let path = jules_path.join("workstreams").join(workstream).join("scheduled.toml");

    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::ScheduleConfigMissing(path.display().to_string())
        } else {
            AppError::config_error(format!("Failed to read {}: {}", path.display(), err))
        }
    })?;
    WorkstreamSchedule::parse_toml(&content)
}

pub fn list_subdirectories(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    Ok(entries)
}
