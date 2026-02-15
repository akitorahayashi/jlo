use std::path::Path;

use crate::domain::AppError;
use crate::ports::{JulesStore, RepositoryFilesystem};

pub struct RequirementPathInfo {
    pub requirement_path_str: String,
}

pub fn validate_requirement_path<W: RepositoryFilesystem + JulesStore + ?Sized>(
    requirement_path: &Path,
    repository: &W,
) -> Result<RequirementPathInfo, AppError> {
    let path_str = requirement_path.to_str().ok_or_else(|| {
        AppError::Validation("Requirement path contains invalid unicode".to_string())
    })?;

    if !repository.file_exists(path_str) {
        return Err(AppError::RequirementFileNotFound(path_str.to_string()));
    }

    let canonical_path = repository.canonicalize(path_str)?;

    let exchange_dir = crate::domain::exchange::paths::exchange_dir(&repository.jules_path());
    let exchange_dir_str = exchange_dir.to_str().ok_or_else(|| {
        AppError::Validation("Exchange path contains invalid unicode".to_string())
    })?;

    let canonical_exchange_dir = repository
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
