use std::error::Error;
use std::fmt::{self, Display};
use std::io;

/// Library-wide error type for jo operations.
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
    /// Jo-managed files have been modified locally.
    ModifiedFiles(Vec<String>),
    /// Role identifier is invalid.
    InvalidRoleId(String),
    /// Version mismatch between installed jo and workspace.
    VersionMismatch { installed: String, workspace: String },
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
            AppError::ModifiedFiles(files) => {
                write!(f, "Modified jo-managed files detected: {}", files.join(", "))
            }
            AppError::InvalidRoleId(id) => {
                write!(f, "Invalid role identifier '{}': must be alphanumeric with hyphens", id)
            }
            AppError::VersionMismatch { installed, workspace } => {
                write!(f, "Version mismatch: jo {} vs workspace {}", installed, workspace)
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
    pub(crate) fn config_error<S: Into<String>>(message: S) -> Self {
        AppError::ConfigError(message.into())
    }

    /// Provide an `io::ErrorKind`-like view for callers expecting legacy behavior.
    pub fn kind(&self) -> io::ErrorKind {
        match self {
            AppError::Io(err) => err.kind(),
            AppError::ConfigError(_) | AppError::InvalidRoleId(_) => io::ErrorKind::InvalidInput,
            AppError::WorkspaceNotFound => io::ErrorKind::NotFound,
            AppError::WorkspaceExists => io::ErrorKind::AlreadyExists,
            AppError::ModifiedFiles(_) | AppError::VersionMismatch { .. } => {
                io::ErrorKind::InvalidData
            }
        }
    }
}
