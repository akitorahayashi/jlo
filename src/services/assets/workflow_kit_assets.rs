use include_dir::{Dir, DirEntry, include_dir};
use std::collections::BTreeSet;
use std::path::{Component, Path};

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::ScaffoldFile;

static WORKFLOWS_REMOTE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/workflows/remote");
static WORKFLOWS_SELF_HOSTED_DIR: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/src/assets/workflows/self-hosted");

#[derive(Debug)]
pub struct WorkflowKitAssets {
    pub files: Vec<ScaffoldFile>,
    pub action_dirs: Vec<String>,
}

pub fn load_workflow_kit(mode: WorkflowRunnerMode) -> Result<WorkflowKitAssets, AppError> {
    let dir = match mode {
        WorkflowRunnerMode::Remote => &WORKFLOWS_REMOTE_DIR,
        WorkflowRunnerMode::SelfHosted => &WORKFLOWS_SELF_HOSTED_DIR,
    };

    let mut files = Vec::new();
    collect_files(dir, dir.path(), &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));

    if files.is_empty() {
        return Err(AppError::config_error(format!(
            "Workflow kit assets are empty for mode '{}'",
            mode.label()
        )));
    }

    let mut action_dirs = BTreeSet::new();
    for file in &files {
        let path = Path::new(&file.path);
        if let Ok(rest) = path.strip_prefix(".github/actions")
            && let Some(Component::Normal(name)) = rest.components().next()
        {
            action_dirs.insert(format!(".github/actions/{}", name.to_string_lossy()));
        }
    }

    Ok(WorkflowKitAssets { files, action_dirs: action_dirs.into_iter().collect() })
}

fn collect_files(
    dir: &Dir,
    base_path: &Path,
    files: &mut Vec<ScaffoldFile>,
) -> Result<(), AppError> {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                let content = file.contents_utf8().ok_or_else(|| {
                    AppError::config_error(format!(
                        "Workflow kit file is not UTF-8: {}",
                        file.path().to_string_lossy()
                    ))
                })?;
                let relative_path = file.path().strip_prefix(base_path).map_err(|_| {
                    AppError::config_error(format!(
                        "Workflow kit file has unexpected path: {}",
                        file.path().to_string_lossy()
                    ))
                })?;
                files.push(ScaffoldFile {
                    path: relative_path.to_string_lossy().to_string(),
                    content: content.to_string(),
                });
            }
            DirEntry::Dir(subdir) => collect_files(subdir, base_path, files)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_kit_assets_load() {
        let remote = load_workflow_kit(WorkflowRunnerMode::Remote).expect("remote assets");
        assert!(!remote.files.is_empty(), "remote kit should have files");

        let self_hosted =
            load_workflow_kit(WorkflowRunnerMode::SelfHosted).expect("self-hosted assets");
        assert!(!self_hosted.files.is_empty(), "self-hosted kit should have files");
    }
}
