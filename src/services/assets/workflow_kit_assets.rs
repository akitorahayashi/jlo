use include_dir::{Dir, DirEntry, include_dir};
use minijinja::{Environment, Value, context};
use std::collections::BTreeSet;
use std::path::{Component, Path};

use crate::domain::{AppError, WorkflowRunnerMode};
use crate::ports::ScaffoldFile;

static WORKFLOWS_ASSET_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/workflows/.github");

#[derive(Debug)]
pub struct WorkflowKitAssets {
    pub files: Vec<ScaffoldFile>,
    pub action_dirs: Vec<String>,
}

/// Helper function for templates to output GitHub Actions expressions.
/// Usage in template: {{ gha_expr("github.ref") }} → ${{ github.ref }}
fn gha_expr(expr: &str) -> String {
    format!("${{{{ {} }}}}", expr)
}

/// Helper function for templates to output raw GitHub Actions expressions.
/// Usage in template: {{ gha_raw("needs.job.outputs.value") }} → ${{ needs.job.outputs.value }}
/// This is identical to gha_expr but the name makes it clearer when used in string contexts.
fn gha_raw(expr: &str) -> String {
    format!("${{{{ {} }}}}", expr)
}

pub fn load_workflow_kit(mode: WorkflowRunnerMode) -> Result<WorkflowKitAssets, AppError> {
    let mut env = Environment::new();
    env.set_keep_trailing_newline(true);

    // Add the gha_expr function to the template environment
    env.add_function("gha_expr", |expr: &str| -> String { gha_expr(expr) });
    env.add_function("gha_raw", |expr: &str| -> String { gha_raw(expr) });

    // Template context based on runner mode
    let runner = match mode {
        WorkflowRunnerMode::Remote => "ubuntu-latest",
        WorkflowRunnerMode::SelfHosted => "self-hosted",
    };

    let ctx = context! {
        runner => runner,
    };

    let mut files = Vec::new();
    collect_and_render_files(
        &WORKFLOWS_ASSET_DIR,
        WORKFLOWS_ASSET_DIR.path(),
        &mut files,
        &env,
        &ctx,
    )?;

    // Prepend .github/ to all paths since we're including the .github directory directly
    for file in &mut files {
        file.path = format!(".github/{}", file.path);
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    if files.is_empty() {
        return Err(AppError::InternalError(format!(
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

fn collect_and_render_files(
    dir: &Dir,
    base_path: &Path,
    files: &mut Vec<ScaffoldFile>,
    env: &Environment,
    ctx: &Value,
) -> Result<(), AppError> {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                let content = file.contents_utf8().ok_or_else(|| {
                    AppError::InternalError(format!(
                        "Workflow kit file is not UTF-8: {}",
                        file.path().to_string_lossy()
                    ))
                })?;

                let file_path = file.path();
                let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                let relative_path = file_path.strip_prefix(base_path).map_err(|_| {
                    AppError::InternalError(format!(
                        "Workflow kit file has unexpected path: {}",
                        file_path.to_string_lossy()
                    ))
                })?;

                // Determine if this is a template file
                let (output_path, rendered_content) = if file_name.ends_with(".j2") {
                    // Render template
                    let template = env.template_from_str(content).map_err(|e| {
                        AppError::InternalError(format!(
                            "Failed to parse template '{}': {}",
                            file_path.to_string_lossy(),
                            e
                        ))
                    })?;
                    let rendered = template.render(ctx).map_err(|e| {
                        AppError::InternalError(format!(
                            "Failed to render template '{}': {}",
                            file_path.to_string_lossy(),
                            e
                        ))
                    })?;
                    // Remove .j2 extension
                    let path_str = relative_path.to_string_lossy();
                    let output = path_str.strip_suffix(".j2").unwrap_or(&path_str).to_string();
                    (output, rendered)
                } else {
                    // Static file - copy as-is
                    (relative_path.to_string_lossy().to_string(), content.to_string())
                };

                files.push(ScaffoldFile { path: output_path, content: rendered_content });
            }
            DirEntry::Dir(subdir) => collect_and_render_files(subdir, base_path, files, env, ctx)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use yamllint_rs::{FileProcessor, ProcessingOptions, Severity};

    #[test]
    fn workflow_kit_assets_load() {
        let remote = load_workflow_kit(WorkflowRunnerMode::Remote).expect("remote assets");
        assert!(!remote.files.is_empty(), "remote kit should have files");

        let self_hosted =
            load_workflow_kit(WorkflowRunnerMode::SelfHosted).expect("self-hosted assets");
        assert!(!self_hosted.files.is_empty(), "self-hosted kit should have files");
    }

    #[test]
    fn workflow_templates_pass_yaml_lint_remote() {
        validate_yaml_lint(WorkflowRunnerMode::Remote);
    }

    #[test]
    fn workflow_templates_pass_yaml_lint_self_hosted() {
        validate_yaml_lint(WorkflowRunnerMode::SelfHosted);
    }

    fn validate_yaml_lint(mode: WorkflowRunnerMode) {
        let kit = load_workflow_kit(mode).expect("should load workflow kit");

        // Configure lint rules appropriate for GitHub Actions workflows
        let mut config = yamllint_rs::config::Config::new();
        // Disable line-length: GitHub Actions expressions often exceed 80 chars
        config.set_rule_enabled("line-length", false);
        // Disable indentation: yamllint-rs seems to have issues with complex GHA if/run blocks
        config.set_rule_enabled("indentation", false);
        // Disable truthy: GHA uses string 'true'/'false' which yamllint dislikes
        config.set_rule_enabled("truthy", false);
        // Disable document-start: '---' is not required for GHA workflows
        config.set_rule_enabled("document-start", false);
        // Disable comments: Allow tight spacing for inline comments
        config.set_rule_enabled("comments", false);

        let processor = FileProcessor::with_config(ProcessingOptions::default(), config);

        let mut errors = Vec::new();

        for file in &kit.files {
            if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
                continue;
            }

            let mut temp = NamedTempFile::new().expect("create temp file");
            temp.write_all(file.content.as_bytes()).expect("write temp file");

            match processor.process_file(temp.path()) {
                Ok(result) => {
                    let issues: Vec<_> = result
                        .issues
                        .iter()
                        .filter(|(issue, _)| issue.severity == Severity::Error)
                        .collect();

                    if !issues.is_empty() {
                        let mut msg = format!("\n  {}:", file.path);
                        for (issue, line) in &issues {
                            msg.push_str(&format!(
                                "\n    L{}: {} - {}",
                                issue.line, issue.message, line
                            ));
                        }
                        errors.push(msg);
                    }
                }
                Err(e) => {
                    errors.push(format!("\n  {}: failed to lint - {}", file.path, e));
                }
            }
        }

        assert!(
            errors.is_empty(),
            "YAML lint errors for {} mode:{}",
            mode.label(),
            errors.join("")
        );
    }
}
