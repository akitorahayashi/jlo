use std::io;

use thiserror::Error;

/// Domain-specific I/O error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    Other,
}

impl From<io::ErrorKind> for IoErrorKind {
    fn from(k: io::ErrorKind) -> Self {
        match k {
            io::ErrorKind::NotFound => IoErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => IoErrorKind::PermissionDenied,
            _ => IoErrorKind::Other,
        }
    }
}

/// Library-wide error type for jlo operations.
#[derive(Debug, Error)]
pub enum AppError {
    /// Underlying I/O failure.
    #[error("I/O error: {message}")]
    Io { message: String, kind: IoErrorKind },

    /// Environment variable not set.
    #[error("Environment variable '{0}' not set")]
    EnvironmentVariableMissing(String),

    /// External tool execution failed.
    #[error("External tool '{tool}' failed: {error}")]
    ExternalToolError { tool: String, error: String },

    /// Jules API error.
    #[error("Jules API error: {message} (Status: {status:?})")]
    JulesApiError { message: String, status: Option<u16> },

    /// General validation error.
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Invalid configuration error.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid path error.
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Missing required argument.
    #[error("Missing argument: {0}")]
    MissingArgument(String),

    /// Exchange directory not found.
    #[error("Exchange directory not found")]
    ExchangeDirectoryNotFound,

    /// Runtime repository integrity issue (e.g. missing version file).
    #[error("Repository integrity error: {0}")]
    RepositoryIntegrity(String),

    /// Runtime repository version mismatch.
    #[error(
        "Repository version ({repository}) is newer than binary version ({binary}). Update the jlo binary."
    )]
    RepositoryVersionMismatch { repository: String, binary: String },

    /// Repository detection failed.
    #[error("Repository detection failed. Set GITHUB_REPOSITORY or run from a git repository.")]
    RepositoryDetectionFailed,

    /// Internal error (bug or unexpected state).
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Asset loading/parsing error.
    #[error("Asset error: {0}")]
    AssetError(String),

    /// `.jlo/` already exists at the target location.
    #[error(".jlo/ already exists")]
    JloAlreadyExists,

    /// No `.jules/` runtime repository found in the current directory.
    #[error("No .jules/ repository found in current directory")]
    JulesNotFound,

    /// Role identifier is invalid.
    #[error("Invalid role identifier '{0}': must be alphanumeric with hyphens or underscores")]
    InvalidRoleId(String),

    /// Setup component identifier is invalid.
    #[error(
        "Invalid setup component identifier '{0}': must be alphanumeric with hyphens, underscores, or periods"
    )]
    InvalidSetupComponentId(String),

    /// Layer identifier is invalid.
    #[error(
        "Invalid layer '{name}': must be one of Narrator, Observers, Decider, Planner, Implementer, Innovators, Integrator"
    )]
    InvalidLayer { name: String },

    /// Role not found (fuzzy match failed).
    #[error("Role '{0}' not found")]
    RoleNotFound(String),

    /// Role already exists at the specified location.
    #[error("Role '{role}' already exists in layer '{layer}'")]
    RoleExists { role: String, layer: String },

    /// Duplicate role requested.
    #[error("Duplicate role '{0}' specified")]
    DuplicateRoleRequest(String),

    /// Setup repository not initialized (.jlo/setup/ missing).
    #[error("Setup not initialized. Run 'jlo init --remote' or 'jlo init --self-hosted' first.")]
    SetupNotInitialized,

    /// Setup config file missing (tools.yml).
    #[error("Setup config file (tools.yml) not found")]
    SetupConfigMissing,

    /// Circular dependency detected during resolution.
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Setup component not found in catalog.
    #[error("Setup component '{name}' not found. Available: {available}")]
    SetupComponentNotFound { name: String, available: String },

    /// Path traversal attempt detected.
    #[error("Path traversal detected: '{0}' escapes repository root")]
    PathTraversal(String),

    /// Invalid setup component metadata.
    #[error("Invalid setup component metadata for '{component}': {reason}")]
    InvalidSetupComponentMetadata { component: String, reason: String },

    /// Malformed setup environment TOML file.
    #[error("Malformed setup environment TOML: {0}")]
    MalformedEnvToml(String),

    /// Control plane config file missing (.jlo/config.toml).
    #[error("Control plane config not found. Create .jlo/config.toml first.")]
    ControlPlaneConfigMissing,

    /// Role not found in config for layer.
    #[error("Role '{role}' not found in config for layer '{layer}'")]
    RoleNotInConfig { role: String, layer: String },

    /// Exchange schedule error.
    #[error(transparent)]
    Schedule(#[from] crate::domain::config::schedule::ScheduleError),

    /// Requirement file not found at path.
    #[error("Requirement file not found: {0}")]
    RequirementFileNotFound(String),

    /// Template creation not supported for single-role layers.
    #[error("Layer '{0}' is single-role and does not support custom roles. Use the built-in role.")]
    SingleRoleLayerTemplate(String),

    /// Prompt assembly failed.
    #[error(transparent)]
    PromptAssembly(#[from] crate::domain::layers::prompt_assemble::PromptAssemblyError),

    /// Git execution failed.
    #[error("Git error running '{command}': {details}")]
    GitError { command: String, details: String },

    /// Parse error.
    #[error("Failed to parse {what}: {details}")]
    ParseError { what: String, details: String },

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    TomlParseError(String),
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io { message: err.to_string(), kind: err.kind().into() }
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        AppError::TomlParseError(err.to_string())
    }
}
