//! Workflow run input boundary.
//!
//! This module centralizes repository-backed input loading required by
//! `workflow run` orchestration.

use crate::app::config;
use crate::domain::{AppError, Schedule};
use crate::ports::{JloStore, RepositoryFilesystem};

/// Load role schedule from control-plane configuration.
pub fn load_schedule(store: &(impl RepositoryFilesystem + JloStore)) -> Result<Schedule, AppError> {
    config::load_schedule(store)
}
