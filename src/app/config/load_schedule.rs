//! Schedule loading from repository.

use crate::domain::config;
use crate::domain::{AppError, Schedule};
use crate::ports::{JloStore, RepositoryFilesystem};

/// Load role schedule from `.jlo/config.toml`.
pub fn load_schedule(store: &(impl RepositoryFilesystem + JloStore)) -> Result<Schedule, AppError> {
    let jlo_path = store.jlo_path();
    let root = jlo_path.parent().ok_or_else(|| {
        AppError::InvalidPath(format!("Invalid .jlo path (missing parent): {}", jlo_path.display()))
    })?;
    let config_path = config::paths::config(root);
    let config_path_str = config_path.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Config path contains invalid unicode: {}",
            config_path.display()
        ))
    })?;

    if !store.file_exists(config_path_str) {
        return Err(AppError::RunConfigMissing);
    }

    let content = store.read_file(config_path_str)?;
    let run_config = config::parse::parse_config_content(&content)?;
    Ok(run_config.schedule().clone())
}
