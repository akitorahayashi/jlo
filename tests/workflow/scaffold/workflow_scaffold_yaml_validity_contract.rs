use super::support;
use crate::harness::TestContext;
use std::fs;

#[test]
fn workflow_templates_parse_with_serde_yaml() {
    for mode in ["remote", "self-hosted"] {
        let ctx = TestContext::new();
        let output_dir = support::generate_workflow_scaffold(&ctx, mode, "parse");

        let files = support::collect_yaml_files(&output_dir);
        assert!(!files.is_empty(), "Generated workflow scaffold produced no YAML files");

        for file in files {
            let content = fs::read_to_string(&file)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", file.display(), e));
            let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(&content);
            assert!(
                result.is_ok(),
                "{} ({} mode) failed to parse with serde_yaml: {}",
                file.display(),
                mode,
                result.unwrap_err()
            );
        }
    }
}

#[test]
fn workflow_templates_pass_yaml_lint_remote() {
    support::validate_yaml_lint("remote");
}

#[test]
fn workflow_templates_pass_yaml_lint_self_hosted() {
    support::validate_yaml_lint("self-hosted");
}

#[test]
fn workflow_templates_validate_structure() {
    let ctx = TestContext::new();
    let output_dir = support::generate_workflow_scaffold(&ctx, "remote", "structure");

    let workflow_path = output_dir.join(".github/workflows/jules-workflows.yml");
    let content = fs::read_to_string(&workflow_path).expect("Failed to read workflow");
    let workflow: serde_yaml::Value = serde_yaml::from_str(&content).expect("Failed to parse YAML");

    let root = workflow.as_mapping().expect("Root should be a mapping");
    assert!(root.contains_key(serde_yaml::Value::from("name")));
    assert!(root.contains_key(serde_yaml::Value::from("on")));
    assert!(root.contains_key(serde_yaml::Value::from("jobs")));
    assert!(root.contains_key(serde_yaml::Value::from("permissions")));

    let on = root
        .get(serde_yaml::Value::from("on"))
        .unwrap()
        .as_mapping()
        .expect("'on' should be mapping");
    assert!(on.contains_key(serde_yaml::Value::from("schedule")));
    assert!(on.contains_key(serde_yaml::Value::from("workflow_dispatch")));

    let jobs = root
        .get(serde_yaml::Value::from("jobs"))
        .unwrap()
        .as_mapping()
        .expect("'jobs' should be mapping");
    for job in [
        "run-narrator",
        "check-schedule",
        "run-observers",
        "run-innovators-1",
        "run-innovators-2",
        "run-decider",
        "run-planner",
        "run-implementer",
    ] {
        assert!(jobs.contains_key(serde_yaml::Value::from(job)), "Missing job '{}'", job);
    }
}
