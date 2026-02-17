//! Mock execution prerequisite validation.

use crate::app::commands::run::RunRuntimeOptions;
use crate::domain::AppError;

/// Validate prerequisites for mock mode.
pub fn validate_mock_prerequisites(_runtime: &RunRuntimeOptions) -> Result<(), AppError> {
    if std::env::var("GH_TOKEN").is_err() {
        return Err(AppError::MissingArgument(
            "Mock mode requires GH_TOKEN environment variable to be set".to_string(),
        ));
    }

    if std::process::Command::new("git").arg("--version").output().is_err() {
        return Err(AppError::ExternalToolError {
            tool: "git".to_string(),
            error: "git is required for mock mode but not found in PATH".to_string(),
        });
    }

    if std::process::Command::new("gh").arg("--version").output().is_err() {
        return Err(AppError::ExternalToolError {
            tool: "gh".to_string(),
            error: "gh CLI is required for mock mode but not found in PATH".to_string(),
        });
    }

    Ok(())
}
