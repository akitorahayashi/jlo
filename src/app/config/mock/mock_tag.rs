//! Mock tag resolution and validation.

use chrono::Utc;

use crate::domain::validation::validate_identifier;
use crate::domain::{AppError, ConfigError};

pub fn resolve_mock_tag() -> Result<String, AppError> {
    let mock_tag = std::env::var("JULES_MOCK_TAG").ok().unwrap_or_else(|| {
        let prefix = if std::env::var("GITHUB_ACTIONS").is_ok() { "mock-ci" } else { "mock-local" };
        let generated = format!("{}-{}", prefix, Utc::now().format("%Y%m%d%H%M%S"));
        println!("Mock tag not set; using {}", generated);
        generated
    });

    if !mock_tag.contains("mock") {
        return Err(ConfigError::Invalid(
            "JULES_MOCK_TAG must include 'mock' to mark mock artifacts.".to_string(),
        )
        .into());
    }
    if !validate_identifier(&mock_tag, false) {
        return Err(ConfigError::Invalid(
            "JULES_MOCK_TAG must be a safe path component (letters, numbers, '-' or '_')."
                .to_string(),
        )
        .into());
    }

    Ok(mock_tag)
}
