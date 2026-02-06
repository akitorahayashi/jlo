//! Component domain model.

use crate::domain::ComponentId;
use serde::Deserialize;

/// Environment variable specification for a component.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EnvSpec {
    /// Variable name.
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Default value (if any).
    #[serde(default)]
    pub default: Option<String>,
}

/// A component that can be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Component name (unique identifier).
    pub name: ComponentId,
    /// Short summary of what this component provides.
    pub summary: String,
    /// Names of components this depends on.
    pub dependencies: Vec<ComponentId>,
    /// Environment variables this component uses.
    pub env: Vec<EnvSpec>,
    /// Installation script content.
    pub script_content: String,
}
