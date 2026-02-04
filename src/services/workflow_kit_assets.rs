use include_dir::{Dir, DirEntry, include_dir};
use minijinja::syntax::SyntaxConfig;
use minijinja::{Environment, UndefinedBehavior, context};
use std::collections::BTreeSet;
use std::path::{Component, Path};

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::ScaffoldFile;

static WORKFLOWS_TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/workflows");

#[derive(Debug)]
pub struct WorkflowKitAssets {
    pub files: Vec<ScaffoldFile>,
    pub action_dirs: Vec<String>,
}

pub fn load_workflow_kit(mode: WorkflowRunnerMode) -> Result<WorkflowKitAssets, AppError> {
    let (runs_on, use_matrix) = match mode {
        WorkflowRunnerMode::Remote => ("ubuntu-latest", false),
        WorkflowRunnerMode::SelfHosted => ("self-hosted", true),
    };

    let context = context! {
        mode => mode.label(),
        runs_on => runs_on,
        use_matrix => use_matrix,
    };

    let mut files = Vec::new();
    let mut seen = BTreeSet::new();

    let templates_root = WORKFLOWS_TEMPLATES_DIR.get_dir(".github").ok_or_else(|| {
        AppError::config_error("Workflow kit templates missing .github directory")
    })?;
    collect_templates(
        templates_root,
        WORKFLOWS_TEMPLATES_DIR.path(),
        &context,
        &mut files,
        &mut seen,
    )?;

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

fn collect_templates(
    dir: &Dir,
    base_path: &Path,
    context: &minijinja::Value,
    files: &mut Vec<ScaffoldFile>,
    seen: &mut BTreeSet<String>,
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
                let relative_path = relative_path.to_string_lossy().to_string();
                let rendered = render_template(content, context, &relative_path)?;
                if rendered.trim().is_empty() {
                    continue;
                }
                if !seen.insert(relative_path.clone()) {
                    return Err(AppError::config_error(format!(
                        "Duplicate workflow kit template path: {}",
                        relative_path
                    )));
                }

                files.push(ScaffoldFile { path: relative_path, content: rendered });
            }
            DirEntry::Dir(subdir) => {
                collect_templates(subdir, base_path, context, files, seen)?;
            }
        }
    }
    Ok(())
}

fn render_template(
    content: &str,
    context: &minijinja::Value,
    path: &str,
) -> Result<String, AppError> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let syntax = SyntaxConfig::builder()
        .block_delimiters("[%", "%]")
        .variable_delimiters("[[[", "]]]")
        .comment_delimiters("[#", "#]")
        .build()
        .map_err(|err| {
            AppError::config_error(format!("Failed to configure workflow template syntax: {}", err))
        })?;
    env.set_syntax(syntax);

    env.add_template(path, content).map_err(|err| {
        AppError::config_error(format!("Failed to load workflow template {}: {}", path, err))
    })?;
    env.get_template(path)
        .map_err(|err| {
            AppError::config_error(format!("Failed to access workflow template {}: {}", path, err))
        })?
        .render(context)
        .map_err(|err| {
            AppError::config_error(format!("Failed to render workflow template {}: {}", path, err))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn workflow_kit_assets_load() {
        let remote = load_workflow_kit(WorkflowRunnerMode::Remote).expect("remote assets");
        assert!(!remote.files.is_empty(), "remote kit should have files");

        let self_hosted =
            load_workflow_kit(WorkflowRunnerMode::SelfHosted).expect("self-hosted assets");
        assert!(!self_hosted.files.is_empty(), "self-hosted kit should have files");
    }

    #[test]
    fn workflow_kit_templates_respect_mode() {
        let remote = load_workflow_kit(WorkflowRunnerMode::Remote).expect("remote assets");
        let self_hosted =
            load_workflow_kit(WorkflowRunnerMode::SelfHosted).expect("self-hosted assets");

        let remote_paths: BTreeSet<String> =
            remote.files.iter().map(|file| file.path.clone()).collect();
        let self_hosted_paths: BTreeSet<String> =
            self_hosted.files.iter().map(|file| file.path.clone()).collect();

        assert!(
            remote_paths.contains(".github/scripts/jules-run-observers-sequential.sh"),
            "remote kit should include sequential observer script"
        );
        assert!(
            !self_hosted_paths.contains(".github/scripts/jules-run-observers-sequential.sh"),
            "self-hosted kit should not include sequential observer script"
        );

        let remote_planner = remote
            .files
            .iter()
            .find(|file| file.path == ".github/workflows/jules-run-planner.yml")
            .expect("remote planner workflow");
        assert!(remote_planner.content.contains("runs-on: ubuntu-latest"));

        let self_hosted_planner = self_hosted
            .files
            .iter()
            .find(|file| file.path == ".github/workflows/jules-run-planner.yml")
            .expect("self-hosted planner workflow");
        assert!(self_hosted_planner.content.contains("runs-on: self-hosted"));
    }
}
