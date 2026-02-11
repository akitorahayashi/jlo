use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer, RequirementHeader};
use crate::ports::WorkspaceStore;
use std::path::PathBuf;

/// Find requirements for a layer in the flat exchange directory.
pub(crate) fn find_issues(
    store: &impl WorkspaceStore,
    layer: Layer,
    _routing_labels: Option<&[String]>,
) -> Result<Vec<PathBuf>, AppError> {
    if layer != Layer::Planner && layer != Layer::Implementer {
        return Err(AppError::Validation("Invalid layer for issue discovery".to_string()));
    }

    let jules_path = store.jules_path();
    let requirements_dir = jules::requirements_dir(&jules_path);

    let requirements_dir_str = match requirements_dir.to_str() {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };

    if !store.file_exists(requirements_dir_str) {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();
    let entries = store.list_dir(requirements_dir_str)?;

    for path in entries {
        let is_yml = path.extension().is_some_and(|ext| ext == "yml" || ext == "yaml");
        if !is_yml {
            continue;
        }

        let requires_deep_analysis = RequirementHeader::read(store, &path)?.requires_deep_analysis;
        let belongs_to_layer = match layer {
            Layer::Planner => requires_deep_analysis,
            Layer::Implementer => !requires_deep_analysis,
            _ => false,
        };
        if belongs_to_layer {
            issues.push(path);
        }
    }

    issues.sort();
    Ok(issues)
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

    fn write_requirement(
        store: &MemoryWorkspaceStore,
        name: &str,
        label: &str,
        requires_deep_analysis: bool,
    ) {
        let content = format!(
            "id: test01\nlabel: {}\nrequires_deep_analysis: {}\nsource_events:\n  - event1\n",
            label, requires_deep_analysis
        );
        let path = format!(".jules/exchange/requirements/{}.yml", name);
        store.write_file(&path, &content).unwrap();
    }

    #[test]
    #[serial]
    fn planner_issue_discovery_filters_by_requires_deep_analysis() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        write_requirement(&store, "requires-planning", "bugs", true);
        write_requirement(&store, "ready-to-implement", "bugs", false);
        write_requirement(&store, "docs-planning", "docs", true);

        let issues = find_issues(&store, Layer::Planner, None).unwrap();

        assert_eq!(issues.len(), 2);
        assert!(issues[0].to_string_lossy().contains("docs-planning.yml"));
        assert!(issues[1].to_string_lossy().contains("requires-planning.yml"));
    }

    #[test]
    #[serial]
    fn implementer_issue_discovery_uses_non_deep_issues() {
        let store = MemoryWorkspaceStore::new();
        setup_workspace(&store);

        write_requirement(&store, "requires-planning", "bugs", true);
        write_requirement(&store, "ready-to-implement", "bugs", false);

        let issues = find_issues(&store, Layer::Implementer, None).unwrap();

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("ready-to-implement.yml"));
    }
}
