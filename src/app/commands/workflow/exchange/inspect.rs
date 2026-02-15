use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::adapters::local_repository::LocalRepositoryAdapter;
use crate::app::configuration::{list_subdirectories, load_schedule};
use crate::domain::AppError;
use crate::domain::repository::paths::jules;
use crate::ports::{JloStore, JulesStore, RepositoryFilesystem};

use super::model::{
    EventItem, EventStateSummary, EventSummary, ExchangeInspectOutput, RequirementItem,
    RequirementSummary, RoleSummary, ScheduleLayerSummary, ScheduleSummary,
};

#[derive(Debug, Clone)]
pub struct ExchangeInspectOptions {}

pub fn execute(_options: ExchangeInspectOptions) -> Result<ExchangeInspectOutput, AppError> {
    let repository = LocalRepositoryAdapter::current()?;

    if !repository.jules_exists() {
        return Err(AppError::JulesNotFound);
    }

    inspect_at(&repository)
}

pub(super) fn inspect_at(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore),
) -> Result<ExchangeInspectOutput, AppError> {
    let jules_path = store.jules_path();
    let exchange_dir = jules::exchange_dir(&jules_path);
    if !store.file_exists(exchange_dir.to_str().unwrap()) {
        return Err(AppError::JulesNotFound);
    }

    let schedule = load_schedule(store)?;
    let schedule_summary = ScheduleSummary {
        version: schedule.version,
        enabled: schedule.enabled,
        observers: ScheduleLayerSummary {
            roles: schedule
                .observers
                .roles
                .iter()
                .map(|r| RoleSummary { name: r.name.clone().into(), enabled: r.enabled })
                .collect(),
        },
    };

    let root = jules_path.parent().unwrap_or(Path::new("."));
    let events = summarize_events(store, root, &exchange_dir)?;
    let requirements = summarize_requirements(store, root, &exchange_dir)?;

    Ok(ExchangeInspectOutput {
        schema_version: 1,
        schedule: schedule_summary,
        events,
        requirements,
    })
}

fn summarize_events(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore),
    root: &Path,
    exchange_dir: &Path,
) -> Result<EventSummary, AppError> {
    let events_dir = exchange_dir.join("events");
    if !store.file_exists(events_dir.to_str().unwrap()) {
        return Err(AppError::Validation(format!(
            "Missing events directory: {}",
            events_dir.display()
        )));
    }

    let mut states = Vec::new();
    let mut pending_files = Vec::new();
    let mut items = Vec::new();

    let state_dirs = list_subdirectories(store, &events_dir)?;

    for state_dir in state_dirs {
        let state_name = state_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let files = list_yml_files(store, &state_dir)?;
        states.push(EventStateSummary { name: state_name.clone(), count: files.len() });

        if state_name == "pending" {
            pending_files = files.iter().map(|path| to_repo_relative(root, path)).collect();
        }

        for path in &files {
            let item = read_event_item(store, root, path, &state_name)?;
            items.push(item);
        }
    }

    items.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(EventSummary { states, pending_files, items })
}

fn summarize_requirements(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore),
    root: &Path,
    exchange_dir: &Path,
) -> Result<RequirementSummary, AppError> {
    let requirements_dir = exchange_dir.join("requirements");
    if !store.file_exists(requirements_dir.to_str().unwrap()) {
        return Err(AppError::Validation(format!(
            "Missing requirements directory: {}",
            requirements_dir.display()
        )));
    }

    let mut items = Vec::new();
    let files = list_yml_files(store, &requirements_dir)?;

    for path in &files {
        let item = read_requirement_item(store, root, path)?;
        items.push(item);
    }

    items.sort_by(|left, right| left.path.cmp(&right.path));
    let count = items.len();

    Ok(RequirementSummary { count, items })
}

fn list_yml_files(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore),
    dir: &Path,
) -> Result<Vec<PathBuf>, AppError> {
    let entries = store.list_dir(dir.to_str().unwrap())?;
    let mut files: Vec<PathBuf> = entries
        .into_iter()
        .filter(|path| path.extension().map(|ext| ext == "yml").unwrap_or(false))
        .collect();
    files.sort();
    Ok(files)
}

fn to_repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string()
}

fn read_event_item(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore),
    root: &Path,
    path: &Path,
    state: &str,
) -> Result<EventItem, AppError> {
    let map = read_yaml_mapping(store, path)?;
    let id = read_required_id(&map, path, "id")?;

    Ok(EventItem { path: to_repo_relative(root, path), state: state.to_string(), id })
}

fn read_requirement_item(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore),
    root: &Path,
    path: &Path,
) -> Result<RequirementItem, AppError> {
    let map = read_yaml_mapping(store, path)?;
    let id = read_required_id(&map, path, "id")?;
    let label = read_required_string(&map, path, "label")?;
    let requires_deep_analysis = read_required_bool(&map, path, "requires_deep_analysis")?;
    let source_events = read_required_string_list(&map, path, "source_events")?;

    Ok(RequirementItem {
        path: to_repo_relative(root, path),
        label,
        requires_deep_analysis,
        id,
        source_events,
    })
}

fn read_yaml_mapping(
    store: &(impl RepositoryFilesystem + JloStore + JulesStore),
    path: &Path,
) -> Result<Mapping, AppError> {
    let content = store.read_file(path.to_str().unwrap())?;
    let value: Value = serde_yaml::from_str(&content).map_err(|err| AppError::ParseError {
        what: path.display().to_string(),
        details: err.to_string(),
    })?;

    match value {
        Value::Mapping(map) => Ok(map),
        _ => {
            Err(AppError::Validation(format!("YAML root must be a mapping in {}", path.display())))
        }
    }
}

fn read_required_string(map: &Mapping, path: &Path, key: &str) -> Result<String, AppError> {
    match map.get(Value::String(key.to_string())) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        Some(Value::String(_)) => Err(AppError::Validation(format!(
            "Field '{}' must be non-empty in {}",
            key,
            path.display()
        ))),
        Some(_) => Err(AppError::Validation(format!(
            "Field '{}' must be a string in {}",
            key,
            path.display()
        ))),
        None => Err(AppError::Validation(format!(
            "Missing required field '{}' in {}",
            key,
            path.display()
        ))),
    }
}

fn read_required_id(map: &Mapping, path: &Path, key: &str) -> Result<String, AppError> {
    let value = read_required_string(map, path, key)?;
    if !is_valid_id(&value) {
        return Err(AppError::Validation(format!(
            "Field '{}' must be 6 lowercase alphanumeric chars in {}",
            key,
            path.display()
        )));
    }
    Ok(value)
}

fn read_required_bool(map: &Mapping, path: &Path, key: &str) -> Result<bool, AppError> {
    match map.get(Value::String(key.to_string())) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(AppError::Validation(format!(
            "Field '{}' must be a boolean in {}",
            key,
            path.display()
        ))),
        None => Err(AppError::Validation(format!(
            "Missing required field '{}' in {}",
            key,
            path.display()
        ))),
    }
}

fn read_required_string_list(
    map: &Mapping,
    path: &Path,
    key: &str,
) -> Result<Vec<String>, AppError> {
    match map.get(Value::String(key.to_string())) {
        Some(Value::Sequence(values)) => {
            let output: Result<Vec<String>, _> = values
                .iter()
                .map(|value| match value {
                    Value::String(text) if !text.trim().is_empty() => Ok(text.clone()),
                    Value::String(_) => Err(AppError::Validation(format!(
                        "Field '{}' must not contain empty strings in {}",
                        key,
                        path.display()
                    ))),
                    _ => Err(AppError::Validation(format!(
                        "Field '{}' must contain strings in {}",
                        key,
                        path.display()
                    ))),
                })
                .collect();

            let output = output?;

            if output.is_empty() {
                return Err(AppError::Validation(format!(
                    "Field '{}' must have entries in {}",
                    key,
                    path.display()
                )));
            }

            for event_id in &output {
                if !is_valid_id(event_id) {
                    return Err(AppError::Validation(format!(
                        "Field '{}' must contain 6 lowercase alphanumeric ids in {}",
                        key,
                        path.display()
                    )));
                }
            }

            Ok(output)
        }
        Some(_) => Err(AppError::Validation(format!(
            "Field '{}' must be a list in {}",
            key,
            path.display()
        ))),
        None => Err(AppError::Validation(format!(
            "Missing required field '{}' in {}",
            key,
            path.display()
        ))),
    }
}

fn is_valid_id(value: &str) -> bool {
    value.len() == 6 && value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn inspect_collects_counts_and_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let jules_path = root.join(".jules");
        let jlo_path = root.join(".jlo");
        let exchange_dir = jules_path.join("exchange");
        fs::create_dir_all(exchange_dir.join("events/pending")).unwrap();
        fs::create_dir_all(exchange_dir.join("events/decided")).unwrap();
        fs::create_dir_all(exchange_dir.join("requirements")).unwrap();

        fs::write(exchange_dir.join("events/pending/one.yml"), "id: abc123\n").unwrap();
        fs::write(exchange_dir.join("events/decided/two.yml"), "id: def456\n").unwrap();
        fs::write(
            exchange_dir.join("requirements/bug-fix.yml"),
            r#"
id: abc123
label: bugs
source_events:
  - abc123
requires_deep_analysis: false
"#,
        )
        .unwrap();

        fs::create_dir_all(&jlo_path).unwrap();
        fs::write(
            jlo_path.join("scheduled.toml"),
            r#"
version = 1
enabled = true
[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
"#,
        )
        .unwrap();

        let store = LocalRepositoryAdapter::new(root.to_path_buf());
        let output = inspect_at(&store).unwrap();

        let pending = output.events.states.iter().find(|state| state.name == "pending").unwrap();
        assert_eq!(pending.count, 1);
        assert_eq!(output.events.pending_files.len(), 1);
        assert_eq!(output.events.items.len(), 2);
        let pending_event =
            output.events.items.iter().find(|item| item.state == "pending").unwrap();
        assert_eq!(pending_event.id, "abc123");
        assert!(pending_event.path.ends_with("events/pending/one.yml"));

        assert_eq!(output.requirements.count, 1);
        assert_eq!(output.requirements.items.len(), 1);
        let req = &output.requirements.items[0];
        assert_eq!(req.id, "abc123");
        assert_eq!(req.label, "bugs");
        assert!(!req.requires_deep_analysis);
        assert_eq!(req.source_events, vec!["abc123".to_string()]);
    }
}
