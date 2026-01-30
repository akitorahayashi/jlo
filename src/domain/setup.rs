//! Setup compiler domain models.

/// Environment variable specification for a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvSpec {
    /// Variable name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Default value (if any).
    pub default: Option<String>,
}

/// A component that can be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Component name (unique identifier).
    pub name: String,
    /// Short summary of what this component provides.
    pub summary: String,
    /// Names of components this depends on.
    pub dependencies: Vec<String>,
    /// Environment variables this component uses.
    pub env: Vec<EnvSpec>,
    /// Installation script content.
    pub script_content: String,
}

/// Configuration for setup script generation.
#[derive(Debug, Clone, Default)]
pub struct SetupConfig {
    /// List of tool names to install.
    pub tools: Vec<String>,
}
