//! Run configuration loading from repository.

use std::path::Path;

use crate::domain::configuration::run_config_parser;
use crate::domain::repository::paths::jlo;
use crate::domain::{AppError, RunConfig};
use crate::ports::RepositoryFilesystem;

/// Load and parse the run configuration from `.jlo/config.toml`.
pub fn load_config<W: RepositoryFilesystem>(
    jules_path: &Path,
    repository: &W,
) -> Result<RunConfig, AppError> {
    let root = jules_path.parent().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Invalid .jules path (missing parent): {}",
            jules_path.display()
        ))
    })?;
    let config_path = jlo::config(root);
    let config_path_str = config_path.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Config path contains invalid unicode: {}",
            config_path.display()
        ))
    })?;

    if !repository.file_exists(config_path_str) {
        return Err(AppError::RunConfigMissing);
    }

    let content = repository.read_file(config_path_str)?;
    run_config_parser::parse_config_content(&content)
}
