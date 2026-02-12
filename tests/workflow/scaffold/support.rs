use crate::harness::TestContext;
use crate::harness::jlo_config;
use std::fs;
use std::path::{Path, PathBuf};
use yamllint_rs::{FileProcessor, ProcessingOptions, Severity};

pub(crate) fn generate_workflow_scaffold(ctx: &TestContext, mode: &str, suffix: &str) -> PathBuf {
    let output_dir = ctx
        .work_dir()
        .join(".tmp/workflow-scaffold-generate/tests")
        .join(format!("{}-{}", mode, suffix));

    jlo_config::ensure_jlo_config(ctx.work_dir());

    ctx.cli()
        .args(["workflow", "generate", mode, "--output-dir"])
        .arg(&output_dir)
        .assert()
        .success();

    output_dir
}

pub(crate) fn collect_yaml_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_yaml_files_into(root, &mut files);
    files
}

fn collect_yaml_files_into(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root)
        .unwrap_or_else(|e| panic!("Failed to read directory {}: {}", root.display(), e));

    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            panic!("Failed to read directory entry in {}: {}", root.display(), e)
        });
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files_into(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "yml" || ext == "yaml")
        {
            files.push(path);
        }
    }
}

pub(crate) fn validate_yaml_lint(mode: &str) {
    let ctx = TestContext::new();
    let output_dir = generate_workflow_scaffold(&ctx, mode, "lint");

    let files = collect_yaml_files(&output_dir);
    assert!(
        !files.is_empty(),
        "Generated workflow scaffold produced no YAML files for {} mode",
        mode
    );

    let mut config = yamllint_rs::config::Config::new();
    config.set_rule_enabled("line-length", false);
    config.set_rule_enabled("indentation", false);
    config.set_rule_enabled("truthy", false);
    config.set_rule_enabled("document-start", false);
    config.set_rule_enabled("comments", false);

    let processor = FileProcessor::with_config(ProcessingOptions::default(), config);

    let mut errors = Vec::new();
    for file in files {
        match processor.process_file(&file) {
            Ok(result) => {
                let issues: Vec<_> = result
                    .issues
                    .iter()
                    .filter(|(issue, _)| issue.severity == Severity::Error)
                    .collect();

                if !issues.is_empty() {
                    let mut msg = format!("\n  {}:", file.display());
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
                errors.push(format!("\n  {}: failed to lint - {}", file.display(), e));
            }
        }
    }

    assert!(errors.is_empty(), "YAML lint errors for {} mode:{}", mode, errors.join(""));
}
