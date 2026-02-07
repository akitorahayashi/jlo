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

    /// Missing required argument.
    #[error("Missing argument: {0}")]
    MissingArgument(String),

    /// Workstream already exists (unlike WorkspaceExists which is for .jules).
    #[error("Workstream '{0}' already exists")]
    WorkstreamExists(String),

    /// Workstream not found.
    #[error("Workstream '{0}' not found")]
    WorkstreamNotFound(String),

    /// Workstreams directory not found.
    #[error("Workstreams directory not found")]
    WorkstreamsDirectoryNotFound,

    /// Workspace integrity issue (e.g. missing version file).
    #[error("Workspace integrity error: {0}")]
    WorkspaceIntegrity(String),

    /// Workspace version mismatch.
    #[error(
        "Workspace version ({workspace}) is newer than binary version ({binary}). Update the jlo binary."
    )]
    WorkspaceVersionMismatch { workspace: String, binary: String },

    /// Repository detection failed.
    #[error("Repository detection failed. Set GITHUB_REPOSITORY or run from a git repository.")]
    RepositoryDetectionFailed,

    /// Internal error (bug or unexpected state).
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Asset loading/parsing error.
    #[error("Asset error: {0}")]
    AssetError(String),

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

    /// Duplicate role requested.
    #[error("Duplicate role '{0}' specified")]
    DuplicateRoleRequest(String),

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

    /// Path traversal attempt detected.
    #[error("Path traversal detected: '{0}' escapes workspace root")]
    PathTraversal(String),

    /// Invalid component metadata.
    #[error("Invalid metadata for '{component}': {reason}")]
    InvalidComponentMetadata { component: String, reason: String },

    /// Malformed env.toml file.
    #[error("Malformed env.toml: {0}")]
    MalformedEnvToml(String),

    /// Run config file missing (.jules/config.toml).
    #[error("Run config not found. Create .jules/config.toml first.")]
    RunConfigMissing,

    /// Role not found in config for layer.
    #[error("Role '{role}' not found in config for layer '{layer}'")]
    RoleNotInConfig { role: String, layer: String },

    /// Workstream schedule file missing.
    #[error("Schedule config not found: {0}")]
    ScheduleConfigMissing(String),

    /// Workstream schedule error.
    #[error(transparent)]
    Schedule(#[from] crate::domain::configuration::schedule::ScheduleError),

    /// Issue file not found at path.
    #[error("Issue file not found: {0}")]
    IssueFileNotFound(String),

    /// Template creation not supported for single-role layers.
    #[error("Layer '{0}' is single-role and does not support custom roles. Use the built-in role.")]
    SingleRoleLayerTemplate(String),

    /// Prompt assembly failed.
    #[error(transparent)]
    PromptAssembly(#[from] crate::domain::PromptAssemblyError),

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
