/// Setup capability error.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error(
        "Invalid setup component identifier '{0}': must be alphanumeric with hyphens, underscores, or periods"
    )]
    InvalidComponentId(String),

    #[error("Setup not initialized. Run 'jlo init --remote' or 'jlo init --self-hosted' first.")]
    NotInitialized,

    #[error("Setup config file (tools.yml) not found")]
    ConfigMissing,

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Setup component '{name}' not found. Available: {available}")]
    ComponentNotFound { name: String, available: String },

    #[error("Invalid setup component metadata for '{component}': {reason}")]
    InvalidComponentMetadata { component: String, reason: String },

    #[error("Malformed setup environment TOML: {0}")]
    MalformedEnvToml(String),
}
