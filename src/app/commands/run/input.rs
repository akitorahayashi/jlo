//! Run command input boundary.
//!
//! This module centralizes repository/environment-backed input loading for
//! `run` execution. Command and layer modules should depend on this boundary
//! instead of importing lower-level readers directly.

use std::path::Path;

use crate::app::config;
use crate::domain::{AppError, MockConfig, RunConfig, RunOptions};
use crate::ports::{Git, RepositoryFilesystem};

/// Load run configuration from `.jlo/config.toml`.
pub fn load_run_config<W: RepositoryFilesystem>(
    jules_path: &Path,
    repository: &W,
) -> Result<RunConfig, AppError> {
    config::load_config(jules_path, repository)
}

/// Validate runtime prerequisites for mock execution.
pub fn validate_mock_prerequisites(options: &RunOptions) -> Result<(), AppError> {
    config::validate_mock_prerequisites(options)
}

/// Load mock execution inputs from repository and environment.
pub fn load_mock_config<W: RepositoryFilesystem>(
    jules_path: &Path,
    options: &RunOptions,
    repository: &W,
) -> Result<MockConfig, AppError> {
    config::load_mock_config(jules_path, options, repository)
}

/// Detect repository source used by session dispatch.
pub fn detect_repository_source(git: &(impl Git + ?Sized)) -> Result<String, AppError> {
    config::detect_repository_source(git)
}
