use crate::domain::identities::validation::validate_safe_path_component;
use crate::domain::{AppError, IssueHeader, Layer};
use crate::ports::WorkspaceStore;
use std::path::{Path, PathBuf};

/// Find issues for a specific workstream and layer.
pub(crate) fn find_issues_for_workstream(
    store: &impl WorkspaceStore,
    workstream: &str,
    layer: Layer,
    routing_labels: Option<&[String]>,
) -> Result<Vec<PathBuf>, AppError> {
    if layer != Layer::Planners && layer != Layer::Implementers {
        return Err(AppError::Validation("Invalid layer for issue discovery".to_string()));
    }

    if !validate_safe_path_component(workstream) {
        return Err(AppError::Validation(format!(
            "Invalid workstream name '{}': must be alphanumeric with hyphens or underscores only",
            workstream
        )));
    }

    let jules_path = store.jules_path();
    let issues_root =
        jules_path.join("workstreams").join(workstream).join("exchange").join("issues");

    if !store.file_exists(issues_root.to_str().unwrap()) {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();
    let routing_labels = resolve_routing_labels(store, &issues_root, routing_labels)?;

    for label in routing_labels {
        let label_dir = issues_root.join(&label);
        if !store.file_exists(label_dir.to_str().unwrap()) {
            continue;
        }

        let entries = store.list_dir(label_dir.to_str().unwrap())?;
        for path in entries {
            let is_issue_file = path.extension().is_some_and(|ext| ext == "yml" || ext == "yaml");
            if !is_issue_file {
                continue;
            }

            let requires_deep_analysis = IssueHeader::read(store, &path)?.requires_deep_analysis;
            let belongs_to_layer = match layer {
                Layer::Planners => requires_deep_analysis,
                Layer::Implementers => !requires_deep_analysis,
                _ => false,
            };
            if belongs_to_layer {
                issues.push(path);
            }
        }
    }

    issues.sort();
    Ok(issues)
}

fn resolve_routing_labels(
    store: &impl WorkspaceStore,
    issues_root: &Path,
    routing_labels: Option<&[String]>,
) -> Result<Vec<String>, AppError> {
    if let Some(labels) = routing_labels {
        let labels: Vec<String> = labels.to_vec();

        if labels.is_empty() {
            return Err(AppError::Validation("Provided routing_labels is empty".to_string()));
        }

        for label in &labels {
            if label.contains("..") || label.contains('/') || label.contains('\\') {
                return Err(AppError::Validation(format!(
                    "Invalid routing label '{}': must not contain path separators or '..'",
                    label
                )));
            }
        }

        return Ok(labels);
    }

    eprintln!("ROUTING_LABELS is not set; discovering labels from {}", issues_root.display());
    let mut discovered = Vec::new();
    let entries = store.list_dir(issues_root.to_str().unwrap())?;
    for path in entries {
        if store.is_dir(path.to_str().unwrap()) {
            discovered.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }

    discovered.sort();
    if discovered.is_empty() {
        return Err(AppError::Validation(format!(
            "No issue label directories found under {}",
            issues_root.display()
        )));
    }

    Ok(discovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory_workspace_store::MemoryWorkspaceStore;
    use crate::ports::WorkspaceStore;
    use serial_test::serial;

    fn setup_workspace(store: &MemoryWorkspaceStore) {
        store.write_version(env!("CARGO_PKG_VERSION")).unwrap();
    }

    fn write_issue(
        store: &MemoryWorkspaceStore,
        label: &str,
        name: &str,
        requires_deep_analysis: bool,
    ) {
        let issue_dir = format!(".jules/workstreams/alpha/exchange/issues/{}", label);
        let content = format!(
            "id: test01\nrequires_deep_analysis: {}\nsource_events:\n  - event1\n",
            requires_deep_analysis
        );
        let path = format!("{}/{}.yml", issue_dir, name);
        store.write_file(&path, &content).unwrap();
    }

    #[test]
    #[serial]
    fn planner_issue_discovery_filters_by_requires_deep_analysis() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        write_issue(&store, "bugs", "requires-planning", true);
        write_issue(&store, "bugs", "ready-to-implement", false);
        write_issue(&store, "docs", "ignored-by-routing", true);

        let routing_labels = vec!["bugs".to_string()];
        let issues =
            find_issues_for_workstream(&store, "alpha", Layer::Planners, Some(&routing_labels))
                .unwrap();

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("requires-planning.yml"));
    }

    #[test]
    #[serial]
    fn implementer_issue_discovery_uses_non_deep_issues() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        write_issue(&store, "bugs", "requires-planning", true);
        write_issue(&store, "bugs", "ready-to-implement", false);

        let routing_labels = vec!["bugs".to_string()];
        let issues =
            find_issues_for_workstream(&store, "alpha", Layer::Implementers, Some(&routing_labels))
                .unwrap();

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("ready-to-implement.yml"));
    }
}
