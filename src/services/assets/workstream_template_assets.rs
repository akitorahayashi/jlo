use include_dir::{Dir, DirEntry, include_dir};
use std::path::Path;

use crate::domain::AppError;
use crate::ports::ScaffoldFile;

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/templates");

pub fn workstream_template_files() -> Result<Vec<ScaffoldFile>, AppError> {
    let workstreams_dir = TEMPLATES_DIR
        .get_dir("workstreams")
        .ok_or_else(|| AppError::Internal { message: "Missing workstream templates directory".into() })?;

    let mut files = Vec::new();
    collect_files(workstreams_dir, workstreams_dir.path(), &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));

    if files.is_empty() {
        return Err(AppError::Internal { message: "Workstream templates directory has no files".into() });
    }

    Ok(files)
}

pub fn workstream_template_content(path: &str) -> Result<String, AppError> {
    let full_path = format!("workstreams/{}", path);
    let file = TEMPLATES_DIR
        .get_file(&full_path)
        .ok_or_else(|| AppError::Internal { message: format!("Missing workstream template {}", path) })?;
    file.contents_utf8().map(|content| content.to_string()).ok_or_else(|| {
        AppError::Internal { message: format!("Workstream template {} is not UTF-8", path) }
    })
}

fn collect_files(dir: &Dir, base_path: &Path, files: &mut Vec<ScaffoldFile>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                if let Some(content) = file.contents_utf8()
                    && let Ok(relative_path) = file.path().strip_prefix(base_path)
                {
                    files.push(ScaffoldFile {
                        path: relative_path.to_string_lossy().to_string(),
                        content: content.to_string(),
                    });
                }
            }
            DirEntry::Dir(subdir) => {
                collect_files(subdir, base_path, files);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workstream_template_files_returns_assets() {
        let files = workstream_template_files().expect("Failed to get workstream template files");
        assert!(!files.is_empty(), "Workstream template files should not be empty");

        // Verify scheduled.toml is present
        assert!(
            files.iter().any(|f| f.path == "scheduled.toml"),
            "scheduled.toml should be present"
        );
    }

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
            matches!(result, Err(AppError::Internal { message: _ })),
            "Should return an internal error for missing file"
        );
    }
}
