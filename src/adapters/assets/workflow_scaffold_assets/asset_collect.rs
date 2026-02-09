use include_dir::{Dir, DirEntry};
use std::path::Path;

use crate::domain::AppError;

#[derive(Debug, Clone)]
pub struct AssetSourceFile {
    pub content: String,
    relative_path: String,
    is_template: bool,
}

impl AssetSourceFile {
    pub fn is_template(&self) -> bool {
        self.is_template
    }

    pub fn template_name(&self) -> &str {
        &self.relative_path
    }

    pub fn output_path(&self) -> String {
        if self.is_template {
            self.relative_path.strip_suffix(".j2").unwrap_or(&self.relative_path).to_string()
        } else {
            self.relative_path.clone()
        }
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }
}

pub fn collect_asset_sources(asset_dir: &Dir) -> Result<Vec<AssetSourceFile>, AppError> {
    let mut files = Vec::new();
    collect_entries(asset_dir, asset_dir.path(), &mut files)?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

fn collect_entries(
    dir: &Dir,
    base_path: &Path,
    files: &mut Vec<AssetSourceFile>,
) -> Result<(), AppError> {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                let content = file.contents_utf8().ok_or_else(|| {
                    AppError::InternalError(format!(
                        "Workflow scaffold file is not UTF-8: {}",
                        file.path().to_string_lossy()
                    ))
                })?;

                let file_path = file.path();
                let relative_path = file_path.strip_prefix(base_path).map_err(|_| {
                    AppError::InternalError(format!(
                        "Workflow scaffold file has unexpected path: {}",
                        file_path.to_string_lossy()
                    ))
                })?;

                let relative_path = relative_path.to_string_lossy().to_string();
                files.push(AssetSourceFile {
                    is_template: relative_path.ends_with(".j2"),
                    content: content.to_string(),
                    relative_path,
                });
            }
            DirEntry::Dir(subdir) => collect_entries(subdir, base_path, files)?,
        }
    }

    Ok(())
}
