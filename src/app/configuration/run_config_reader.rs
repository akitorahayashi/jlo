//! Run configuration loading from repository.

use std::path::Path;

use crate::domain::configuration::run_config_parser;
use crate::domain::workspace::paths::jlo;
use crate::domain::{AppError, RunConfig};
use crate::ports::RepositoryFilesystemPort;

/// Load and parse the run configuration from `.jlo/config.toml`.
pub fn load_config<W: RepositoryFilesystemPort>(
    jules_path: &Path,
    workspace: &W,
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

    if !workspace.file_exists(config_path_str) {
        return Err(AppError::RunConfigMissing);
    }

    let content = workspace.read_file(config_path_str)?;
    run_config_parser::parse_config_content(&content)
}
