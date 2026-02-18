//! Run configuration loading from repository.

use std::path::Path;

use crate::domain::config;
use crate::domain::{AppError, ControlPlaneConfig};
use crate::ports::RepositoryFilesystem;

/// Load and parse the run configuration from `.jlo/config.toml`.
pub fn load_config<W: RepositoryFilesystem>(
    jules_path: &Path,
    repository: &W,
) -> Result<ControlPlaneConfig, AppError> {
    let root = jules_path.parent().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Invalid .jules path (missing parent): {}",
            jules_path.display()
        ))
    })?;
    let config_path = config::paths::config(root);
    let config_path_str = config_path.to_str().ok_or_else(|| {
        AppError::InvalidPath(format!(
            "Config path contains invalid unicode: {}",
            config_path.display()
        ))
    })?;

    if !repository.file_exists(config_path_str) {
        return Err(AppError::ControlPlaneConfigMissing);
    }

    let content = repository.read_file(config_path_str)?;
    config::parse::parse_config_content(&content)
}
