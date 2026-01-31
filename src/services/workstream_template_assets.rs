use include_dir::{Dir, DirEntry, include_dir};

use crate::domain::AppError;
use crate::ports::ScaffoldFile;

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/templates");

pub fn workstream_template_files() -> Result<Vec<ScaffoldFile>, AppError> {
    let workstreams_dir = TEMPLATES_DIR
        .get_dir("workstreams")
        .ok_or_else(|| AppError::config_error("Missing workstream templates directory"))?;

    let mut files = Vec::new();
    collect_files(workstreams_dir, "", &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));

    if files.is_empty() {
        return Err(AppError::config_error("Workstream templates directory has no files"));
    }

    Ok(files)
}

fn collect_files(dir: &Dir, prefix: &str, files: &mut Vec<ScaffoldFile>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                if let Some(content) = file.contents_utf8() {
                    let path =
                        format!("{}{}", prefix, file.path().file_name().unwrap().to_string_lossy());
                    files.push(ScaffoldFile { path, content: content.to_string() });
                }
            }
            DirEntry::Dir(subdir) => {
                let name = subdir.path().file_name().unwrap().to_string_lossy();
                let next_prefix = format!("{}{}/", prefix, name);
                collect_files(subdir, &next_prefix, files);
            }
        }
    }
}
