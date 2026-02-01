//! Application configuration and DTOs.

use serde::Deserialize;

/// Configuration for setup script generation.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SetupConfig {
    /// List of tool names to install.
    #[serde(default)]
    pub tools: Vec<String>,
}
