//! Mock tag resolution and validation.

use chrono::Utc;

use crate::domain::AppError;
use crate::domain::identifiers::validation::validate_safe_path_component;

pub fn resolve_mock_tag() -> Result<String, AppError> {
    let mock_tag = std::env::var("JULES_MOCK_TAG").ok().unwrap_or_else(|| {
        let prefix = if std::env::var("GITHUB_ACTIONS").is_ok() { "mock-ci" } else { "mock-local" };
        let generated = format!("{}-{}", prefix, Utc::now().format("%Y%m%d%H%M%S"));
        println!("Mock tag not set; using {}", generated);
        generated
    });

    if !mock_tag.contains("mock") {
        return Err(AppError::InvalidConfig(
            "JULES_MOCK_TAG must include 'mock' to mark mock artifacts.".to_string(),
        ));
    }
    if !validate_safe_path_component(&mock_tag) {
        return Err(AppError::InvalidConfig(
            "JULES_MOCK_TAG must be a safe path component (letters, numbers, '-' or '_')."
                .to_string(),
        ));
    }

    Ok(mock_tag)
}
