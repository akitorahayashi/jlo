//! Application configuration and DTOs.

use crate::domain::EnvSpec;
use serde::Deserialize;

/// Configuration for setup script generation.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SetupConfig {
    /// List of tool names to install.
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Metadata parsed from meta.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentMeta {
    /// Component name (defaults to directory name if missing).
    pub name: Option<String>,
    /// Short summary.
    #[serde(default)]
    pub summary: String,
    /// Dependencies list.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Environment specifications.
    #[serde(default)]
    pub env: Vec<EnvSpec>,
}
