use std::io;

use thiserror::Error;

/// Library-wide error type for jlo operations.
#[derive(Debug, Error)]
pub enum AppError {
    /// Underlying I/O failure.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Configuration or environment issue.
    #[error("{0}")]
    Configuration(String),

    /// Workspace already exists at the target location.
    #[error(".jules/ workspace already exists")]
    WorkspaceExists,

    /// No .jules/ workspace found in the current directory.
    #[error("No .jules/ workspace found in current directory")]
    WorkspaceNotFound,

    /// Role identifier is invalid.
    #[error("Invalid role identifier '{0}': must be alphanumeric with hyphens or underscores")]
    InvalidRoleId(String),

    /// Component identifier is invalid.
    #[error(
        "Invalid component identifier '{0}': must be alphanumeric with hyphens, underscores, or periods"
    )]
    InvalidComponentId(String),

    /// Layer identifier is invalid.
    #[error(
        "Invalid layer '{name}': must be one of Narrator, Observers, Deciders, Planners, Implementers"
    )]
    InvalidLayer { name: String },

    /// Role not found (fuzzy match failed).
    #[error("Role '{0}' not found")]
    RoleNotFound(String),

    /// Role already exists at the specified location.
    #[error("Role '{role}' already exists in layer '{layer}'")]
    RoleExists { role: String, layer: String },

    /// Setup workspace not initialized (.jules/setup/ missing).
    #[error("Setup not initialized. Run 'jlo setup init' first.")]
    SetupNotInitialized,

    /// Setup config file missing (tools.yml).
    #[error("Setup config file (tools.yml) not found")]
    SetupConfigMissing,

    /// Circular dependency detected during resolution.
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Component not found in catalog.
    #[error("Component '{name}' not found. Available: {available}")]
    ComponentNotFound { name: String, available: String },

    /// Invalid component metadata.
    #[error("Invalid metadata for '{component}': {reason}")]
    InvalidComponentMetadata { component: String, reason: String },

    /// Malformed env.toml file.
    #[error("Malformed env.toml: {0}")]
    MalformedEnvToml(String),

    /// Run config file missing (.jules/config.toml).
    #[error("Run config not found. Create .jules/config.toml first.")]
    RunConfigMissing,

    /// Run config error.
    #[error(transparent)]
    RunConfig(#[from] crate::domain::run_config::RunConfigError),

    /// Role not found in config for layer.
    #[error("Role '{role}' not found in config for layer '{layer}'")]
    RoleNotInConfig { role: String, layer: String },

    /// Workstream schedule file missing.
    #[error("Schedule config not found: {0}")]
    ScheduleConfigMissing(String),

    /// Workstream schedule error.
    #[error(transparent)]
    Schedule(#[from] crate::domain::schedule::ScheduleError),

    /// Issue file not found at path.
    #[error("Issue file not found: {0}")]
    IssueFileNotFound(String),

    /// Template creation not supported for single-role layers.
    #[error("Layer '{0}' is single-role and does not support custom roles. Use the built-in role.")]
    SingleRoleLayerTemplate(String),

    /// Prompt assembly failed.
    #[error("Prompt assembly failed: {0}")]
    PromptAssemblyError(String),

    /// Git execution failed.
    #[error("Git error running '{command}': {details}")]
    GitError { command: String, details: String },

    /// Parse error.
    #[error("Failed to parse {what}: {details}")]
    ParseError { what: String, details: String },

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    TomlParseError(#[from] toml::de::Error),
}

impl AppError {
    pub fn config_error<S: Into<String>>(message: S) -> Self {
        AppError::Configuration(message.into())
    }

    /// Provide an `io::ErrorKind`-like view for callers expecting legacy behavior.
    pub fn kind(&self) -> io::ErrorKind {
        match self {
            AppError::Io(err) => err.kind(),
            AppError::Configuration(_)
            | AppError::InvalidRoleId(_)
            | AppError::InvalidComponentId(_)
            | AppError::InvalidLayer { .. }
            | AppError::RoleNotFound(_)
            | AppError::CircularDependency(_)
            | AppError::InvalidComponentMetadata { .. }
            | AppError::MalformedEnvToml(_)
            | AppError::RunConfig(_)
            | AppError::RoleNotInConfig { .. }
            | AppError::Schedule(_)
            | AppError::SingleRoleLayerTemplate(_)
            | AppError::PromptAssemblyError(_)
            | AppError::ParseError { .. }
            | AppError::TomlParseError(_) => io::ErrorKind::InvalidInput,
            AppError::WorkspaceNotFound
            | AppError::SetupNotInitialized
            | AppError::SetupConfigMissing
            | AppError::ComponentNotFound { .. }
            | AppError::RunConfigMissing
            | AppError::ScheduleConfigMissing(_)
            | AppError::IssueFileNotFound(_) => io::ErrorKind::NotFound,
            AppError::WorkspaceExists | AppError::RoleExists { .. } => io::ErrorKind::AlreadyExists,
            AppError::GitError { .. } => io::ErrorKind::Other,
        }
    }
}
