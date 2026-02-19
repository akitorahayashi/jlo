use crate::domain::{AppError, Layer, RequirementHeader};
use crate::ports::{JulesStore, RepositoryFilesystem};
use std::path::PathBuf;

/// Find requirements for a layer in the flat exchange directory.
pub(crate) fn find_requirements(
    store: &(impl RepositoryFilesystem + JulesStore),
    layer: Layer,
) -> Result<Vec<PathBuf>, AppError> {
    if layer != Layer::Planner && layer != Layer::Implementer {
        return Err(AppError::Validation("Invalid layer for requirement discovery".to_string()));
    }

    let jules_path = store.jules_path();
    let requirements_dir =
        crate::domain::exchange::requirements::paths::requirements_dir(&jules_path);

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
        let is_yml = path.extension().is_some_and(|ext| ext == "yml");
        if !is_yml {
            continue;
        }

        let path_str = path
            .to_str()
            .ok_or_else(|| AppError::Validation(format!("Invalid path: {}", path.display())))?;
        let content = store.read_file(path_str)?;
        let implementation_ready = RequirementHeader::parse(&content)
            .map_err(|err| match err {
                AppError::ParseError { details, .. } => {
                    AppError::ParseError { what: path_str.to_string(), details }
                }
                other => other,
            })?
            .implementation_ready;
        let belongs_to_layer = match layer {
            Layer::Planner => !implementation_ready,
            Layer::Implementer => implementation_ready,
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
    use crate::ports::{JulesStore, RepositoryFilesystem};
    use crate::testing::TestStore;
    use serial_test::serial;

    fn setup_workspace(store: &TestStore) {
        store.jules_write_version(env!("CARGO_PKG_VERSION")).unwrap();
    }

    fn write_requirement(store: &TestStore, name: &str, label: &str, implementation_ready: bool) {
        let content = format!(
            "id: test01\nlabel: {}\nimplementation_ready: {}\nsource_events:\n  - event1\n",
            label, implementation_ready
        );
        let path = format!(".jules/exchange/requirements/{}.yml", name);
        store.write_file(&path, &content).unwrap();
    }

    #[test]
    #[serial]
    fn planner_issue_discovery_filters_by_not_implementation_ready() {
        let store = TestStore::new();
        setup_workspace(&store);

        write_requirement(&store, "requires-planning", "bugs", false);
        write_requirement(&store, "ready-to-implement", "bugs", true);
        write_requirement(&store, "docs-planning", "docs", false);

        let issues = find_requirements(&store, Layer::Planner).unwrap();

        assert_eq!(issues.len(), 2);
        assert!(issues[0].to_string_lossy().contains("docs-planning.yml"));
        assert!(issues[1].to_string_lossy().contains("requires-planning.yml"));
    }

    #[test]
    #[serial]
    fn implementer_issue_discovery_uses_non_deep_issues() {
        let store = TestStore::new();
        setup_workspace(&store);

        write_requirement(&store, "requires-planning", "bugs", false);
        write_requirement(&store, "ready-to-implement", "bugs", true);

        let issues = find_requirements(&store, Layer::Implementer).unwrap();

        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string_lossy().contains("ready-to-implement.yml"));
    }
}
