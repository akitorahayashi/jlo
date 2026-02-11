use std::path::Path;

use crate::domain::AppError;
use crate::domain::workspace::paths::jules;
use crate::ports::WorkspaceStore;

pub(crate) struct RequirementPathInfo {
    pub(crate) requirement_path_str: String,
}

pub(crate) fn validate_requirement_path<W: WorkspaceStore>(
    requirement_path: &Path,
    workspace: &W,
) -> Result<RequirementPathInfo, AppError> {
    let path_str = requirement_path.to_str().ok_or_else(|| {
        AppError::Validation("Requirement path contains invalid unicode".to_string())
    })?;

    if !workspace.file_exists(path_str) {
        return Err(AppError::RequirementFileNotFound(path_str.to_string()));
    }

    let canonical_path = workspace.canonicalize(path_str)?;

    let exchange_dir = jules::exchange_dir(&workspace.jules_path());
    let exchange_dir_str = exchange_dir.to_str().ok_or_else(|| {
        AppError::Validation("Exchange path contains invalid unicode".to_string())
    })?;

    let canonical_exchange_dir = workspace
        .canonicalize(exchange_dir_str)
        .map_err(|_| AppError::ExchangeDirectoryNotFound)?;

    let has_requirements_component =
        canonical_path.components().any(|c| c.as_os_str() == "requirements");
    if !canonical_path.starts_with(&canonical_exchange_dir) || !has_requirements_component {
        return Err(AppError::Validation(format!(
            "Requirement file must be within {}/requirements/",
            canonical_exchange_dir.display()
        )));
    }

    Ok(RequirementPathInfo { requirement_path_str: path_str.to_string() })
}
