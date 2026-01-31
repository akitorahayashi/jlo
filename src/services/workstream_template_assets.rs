use include_dir::{Dir, DirEntry, include_dir};
use std::path::Path;

use crate::domain::AppError;
use crate::ports::ScaffoldFile;

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/templates");

pub fn workstream_template_files() -> Result<Vec<ScaffoldFile>, AppError> {
    let workstreams_dir = TEMPLATES_DIR
        .get_dir("workstreams")
        .ok_or_else(|| AppError::config_error("Missing workstream templates directory"))?;

    let mut files = Vec::new();
    collect_files(workstreams_dir, workstreams_dir.path(), &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));

    if files.is_empty() {
        return Err(AppError::config_error("Workstream templates directory has no files"));
    }

    Ok(files)
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
