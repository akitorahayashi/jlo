//! Application configuration for rs-cli-tmpl.

use crate::error::AppError;
use std::path::PathBuf;

/// Application-wide configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Base path for storage operations.
    pub storage_path: PathBuf,
}

impl Config {
    /// Create a new configuration with custom storage path.
    pub fn with_path(path: PathBuf) -> Self {
        Self { storage_path: path }
    }

    /// Create configuration using the HOME-based config directory.
    ///
    /// Uses $HOME/.config/rs-cli-tmpl for consistency across platforms and tests.
    pub fn new_default() -> Result<Self, AppError> {
        let home = std::env::var("HOME")
            .map_err(|_| AppError::config_error("HOME environment variable not set"))?;
        let storage_path = PathBuf::from(home).join(".config").join("rs-cli-tmpl");
        Ok(Self { storage_path })
    }
}
