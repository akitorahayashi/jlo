use include_dir::{Dir, include_dir};

use crate::domain::AppError;

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/templates");

pub fn workstream_template_content(path: &str) -> Result<String, AppError> {
    let full_path = format!("workstreams/{}", path);
    let file = TEMPLATES_DIR
        .get_file(&full_path)
        .ok_or_else(|| AppError::InternalError(format!("Missing workstream template {}", path)))?;
    file.contents_utf8().map(|content| content.to_string()).ok_or_else(|| {
        AppError::InternalError(format!("Workstream template {} is not UTF-8", path))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workstream_template_content_returns_content() {
        let content = workstream_template_content("scheduled.toml")
            .expect("Failed to get content for scheduled.toml");
        assert!(content.contains("version = 1"), "Content should contain 'version = 1'");
    }

    #[test]
    fn test_workstream_template_content_returns_error_for_missing_file() {
        let result = workstream_template_content("non_existent_file.toml");
        assert!(
            matches!(result, Err(AppError::InternalError(_))),
            "Should return an internal error for missing file"
        );
    }
}
