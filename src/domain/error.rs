use std::error::Error;
use std::fmt::{self, Display};
use std::io;

use super::Layer;

/// Library-wide error type for jlo operations.
#[derive(Debug)]
pub enum AppError {
    /// Underlying I/O failure.
    Io(io::Error),
    /// Configuration or environment issue.
    ConfigError(String),
    /// Workspace already exists at the target location.
    WorkspaceExists,
    /// No .jules/ workspace found in the current directory.
    WorkspaceNotFound,
    /// Role identifier is invalid.
    InvalidRoleId(String),
    /// Layer identifier is invalid.
    InvalidLayer(String),
    /// Role not found (fuzzy match failed).
    RoleNotFound(String),
    /// Role already exists at the specified location.
    RoleExists { role: String, layer: String },
    /// Clipboard operation failed.
    ClipboardError(String),
    /// Setup workspace not initialized (.jules/setup/ missing).
    SetupNotInitialized,
    /// Setup config file missing (tools.yml).
    SetupConfigMissing,
    /// Circular dependency detected during resolution.
    CircularDependency(Vec<String>),
    /// Component not found in catalog.
    ComponentNotFound { name: String, available: Vec<String> },
    /// Invalid component metadata.
    InvalidComponentMetadata { component: String, reason: String },
    /// Malformed env.toml file.
    MalformedEnvToml(String),
    /// Run config file missing (.jules/config.toml).
    RunConfigMissing,
    /// Run config file is malformed.
    RunConfigInvalid(String),
    /// Role not found in config for layer.
    RoleNotInConfig { role: String, layer: String },

    /// Issue file not found at path.
    IssueFileNotFound(String),
    /// Template creation not supported for single-role layers.
    SingleRoleLayerTemplate(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "{}", err),
            AppError::ConfigError(message) => write!(f, "{message}"),
            AppError::WorkspaceExists => {
                write!(f, ".jules/ workspace already exists")
            }
            AppError::WorkspaceNotFound => {
                write!(f, "No .jules/ workspace found in current directory")
            }
            AppError::InvalidRoleId(id) => {
                write!(
                    f,
                    "Invalid role identifier '{}': must be alphanumeric with hyphens or underscores",
                    id
                )
            }
            AppError::InvalidLayer(name) => {
                let available: Vec<&str> =
                    Layer::ALL.iter().map(|layer| layer.dir_name()).collect();
                write!(f, "Invalid layer '{}': must be one of {}", name, available.join(", "))
            }
            AppError::RoleNotFound(query) => {
                write!(f, "Role '{}' not found", query)
            }
            AppError::RoleExists { role, layer } => {
                write!(f, "Role '{}' already exists in layer '{}'", role, layer)
            }
            AppError::ClipboardError(msg) => {
                write!(f, "Clipboard error: {}", msg)
            }
            AppError::SetupNotInitialized => {
                write!(f, "Setup not initialized. Run 'jlo setup init' first.")
            }
            AppError::SetupConfigMissing => {
                write!(f, "Setup config file (tools.yml) not found")
            }
            AppError::CircularDependency(path) => {
                write!(f, "Circular dependency detected: {}", path.join(" -> "))
            }
            AppError::ComponentNotFound { name, available } => {
                write!(f, "Component '{}' not found. Available: {}", name, available.join(", "))
            }
            AppError::InvalidComponentMetadata { component, reason } => {
                write!(f, "Invalid metadata for '{}': {}", component, reason)
            }
            AppError::MalformedEnvToml(location) => {
                write!(f, "Malformed env.toml: {}", location)
            }
            AppError::RunConfigMissing => {
                write!(f, "Run config not found. Create .jules/config.toml first.")
            }
            AppError::RunConfigInvalid(reason) => {
                write!(f, "Invalid run config: {}", reason)
            }
            AppError::RoleNotInConfig { role, layer } => {
                write!(f, "Role '{}' not found in config for layer '{}'", role, layer)
            }

            AppError::IssueFileNotFound(path) => {
                write!(f, "Issue file not found: {}", path)
            }
            AppError::SingleRoleLayerTemplate(layer) => {
                write!(
                    f,
                    "Layer '{}' is single-role and does not support custom templates. Use the built-in role.",
                    layer
                )
            }
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        AppError::Io(value)
    }
}

impl AppError {
    pub fn config_error<S: Into<String>>(message: S) -> Self {
        AppError::ConfigError(message.into())
    }

    /// Provide an `io::ErrorKind`-like view for callers expecting legacy behavior.
    pub fn kind(&self) -> io::ErrorKind {
        match self {
            AppError::Io(err) => err.kind(),
            AppError::ConfigError(_)
            | AppError::InvalidRoleId(_)
            | AppError::InvalidLayer(_)
            | AppError::RoleNotFound(_)
            | AppError::CircularDependency(_)
            | AppError::InvalidComponentMetadata { .. }
            | AppError::MalformedEnvToml(_)
            | AppError::RunConfigInvalid(_)
            | AppError::RoleNotInConfig { .. }
            | AppError::SingleRoleLayerTemplate(_) => io::ErrorKind::InvalidInput,
            AppError::WorkspaceNotFound
            | AppError::SetupNotInitialized
            | AppError::SetupConfigMissing
            | AppError::ComponentNotFound { .. }
            | AppError::RunConfigMissing
            | AppError::IssueFileNotFound(_) => io::ErrorKind::NotFound,
            AppError::WorkspaceExists | AppError::RoleExists { .. } => io::ErrorKind::AlreadyExists,
            AppError::ClipboardError(_) => io::ErrorKind::Other,
        }
    }
}
