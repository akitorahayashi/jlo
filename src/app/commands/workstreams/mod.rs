use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::domain::AppError;
use crate::services::adapters::workstream_schedule_filesystem::{list_subdirectories, load_schedule};

#[derive(Debug, Clone)]
pub enum WorkstreamInspectFormat {
    Json,
    Yaml,
}

#[derive(Debug, Clone)]
pub struct WorkstreamInspectOptions {
    pub workstream: String,
    pub format: WorkstreamInspectFormat,
}

#[derive(Debug, Serialize)]
pub struct WorkstreamInspectOutput {
    pub schema_version: u32,
    pub workstream: String,
    pub schedule: ScheduleSummary,
    pub events: EventSummary,
    pub issues: IssueSummary,
}

#[derive(Debug, Serialize)]
pub struct ScheduleSummary {
    pub version: u32,
    pub enabled: bool,
    pub observers: ScheduleLayerSummary,
    pub deciders: ScheduleLayerSummary,
}

#[derive(Debug, Serialize)]
pub struct ScheduleLayerSummary {
    pub roles: Vec<RoleSummary>,
}

#[derive(Debug, Serialize)]
pub struct RoleSummary {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct EventSummary {
    pub states: Vec<EventStateSummary>,
    pub pending_files: Vec<String>,
    pub items: Vec<EventItem>,
}

#[derive(Debug, Serialize)]
pub struct EventStateSummary {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct EventItem {
    pub path: String,
    pub state: String,
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct IssueSummary {
    pub labels: Vec<IssueLabelSummary>,
    pub items: Vec<IssueItem>,
}

#[derive(Debug, Serialize)]
pub struct IssueLabelSummary {
    pub name: String,
    pub count: usize,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IssueItem {
    pub path: String,
    pub label: String,
    pub requires_deep_analysis: bool,
    pub id: String,
    pub source_events: Vec<String>,
}

pub fn inspect(
    jules_path: &Path,
    options: WorkstreamInspectOptions,
) -> Result<WorkstreamInspectOutput, AppError> {
    let ws_dir = jules_path.join("workstreams").join(&options.workstream);
    if !ws_dir.exists() {
        return Err(AppError::config_error(format!(
            "Workstream '{}' not found",
            options.workstream
        )));
    }

    let schedule = load_schedule(jules_path, &options.workstream)?;
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
        deciders: ScheduleLayerSummary {
            roles: schedule
                .deciders
                .roles
                .iter()
                .map(|r| RoleSummary { name: r.name.clone().into(), enabled: r.enabled })
                .collect(),
        },
    };

    let root = jules_path.parent().unwrap_or(Path::new("."));
    let events = summarize_events(root, &ws_dir)?;
    let issues = summarize_issues(root, &ws_dir)?;

    Ok(WorkstreamInspectOutput {
        schema_version: 1,
        workstream: options.workstream,
        schedule: schedule_summary,
        events,
        issues,
    })
}

fn summarize_events(root: &Path, ws_dir: &Path) -> Result<EventSummary, AppError> {
    let events_dir = ws_dir.join("exchange").join("events");
    if !events_dir.exists() {
        return Err(AppError::config_error(format!(
            "Missing events directory: {}",
            events_dir.display()
        )));
    }

    let mut states = Vec::new();
    let mut pending_files = Vec::new();
    let mut items = Vec::new();

    let state_dirs = list_subdirectories(&events_dir)?;

    for state_dir in state_dirs {
        let state_name = state_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let files = list_yml_files(&state_dir)?;
        states.push(EventStateSummary { name: state_name.clone(), count: files.len() });

        if state_name == "pending" {
            pending_files = files.iter().map(|path| to_repo_relative(root, path)).collect();
        }

        for path in &files {
            let item = read_event_item(root, path, &state_name)?;
            items.push(item);
        }
    }

    items.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(EventSummary { states, pending_files, items })
}

fn summarize_issues(root: &Path, ws_dir: &Path) -> Result<IssueSummary, AppError> {
    let issues_dir = ws_dir.join("exchange").join("issues");
    if !issues_dir.exists() {
        return Err(AppError::config_error(format!(
            "Missing issues directory: {}",
            issues_dir.display()
        )));
    }

    let mut labels = Vec::new();
    let mut items = Vec::new();
    let label_dirs = list_subdirectories(&issues_dir)?;

    for label_dir in label_dirs {
        let label_name = label_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let files = list_yml_files(&label_dir)?;
        let rel_files = files.iter().map(|path| to_repo_relative(root, path)).collect::<Vec<_>>();
        labels.push(IssueLabelSummary {
            name: label_name.clone(),
            count: rel_files.len(),
            files: rel_files,
        });

        for path in &files {
            let item = read_issue_item(root, path, &label_name)?;
            items.push(item);
        }
    }

    items.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(IssueSummary { labels, items })
}

fn list_yml_files(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "yml").unwrap_or(false))
        .collect();
    files.sort();
    Ok(files)
}

fn to_repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string()
}

fn read_event_item(root: &Path, path: &Path, state: &str) -> Result<EventItem, AppError> {
    let map = read_yaml_mapping(path)?;
    let id = read_required_id(&map, path, "id")?;

    Ok(EventItem { path: to_repo_relative(root, path), state: state.to_string(), id })
}

fn read_issue_item(root: &Path, path: &Path, label: &str) -> Result<IssueItem, AppError> {
    let map = read_yaml_mapping(path)?;
    let id = read_required_id(&map, path, "id")?;
    let requires_deep_analysis = read_required_bool(&map, path, "requires_deep_analysis")?;
    let source_events = read_required_string_list(&map, path, "source_events")?;

    Ok(IssueItem {
        path: to_repo_relative(root, path),
        label: label.to_string(),
        requires_deep_analysis,
        id,
        source_events,
    })
}

fn read_yaml_mapping(path: &Path) -> Result<Mapping, AppError> {
    let content = fs::read_to_string(path)?;
    let value: Value = serde_yaml::from_str(&content).map_err(|err| {
        AppError::config_error(format!("Invalid YAML in {}: {}", path.display(), err))
    })?;

    match value {
        Value::Mapping(map) => Ok(map),
        _ => Err(AppError::config_error(format!(
            "YAML root must be a mapping in {}",
            path.display()
        ))),
    }
}

fn read_required_string(map: &Mapping, path: &Path, key: &str) -> Result<String, AppError> {
    match map.get(Value::String(key.to_string())) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        Some(Value::String(_)) => Err(AppError::config_error(format!(
            "Field '{}' must be non-empty in {}",
            key,
            path.display()
        ))),
        Some(_) => Err(AppError::config_error(format!(
            "Field '{}' must be a string in {}",
            key,
            path.display()
        ))),
        None => Err(AppError::config_error(format!(
            "Missing required field '{}' in {}",
            key,
            path.display()
        ))),
    }
}

fn read_required_id(map: &Mapping, path: &Path, key: &str) -> Result<String, AppError> {
    let value = read_required_string(map, path, key)?;
    if !is_valid_id(&value) {
        return Err(AppError::config_error(format!(
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
        Some(_) => Err(AppError::config_error(format!(
            "Field '{}' must be a boolean in {}",
            key,
            path.display()
        ))),
        None => Err(AppError::config_error(format!(
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
                    Value::String(_) => Err(AppError::config_error(format!(
                        "Field '{}' must not contain empty strings in {}",
                        key,
                        path.display()
                    ))),
                    _ => Err(AppError::config_error(format!(
                        "Field '{}' must contain strings in {}",
                        key,
                        path.display()
                    ))),
                })
                .collect();

            let output = output?;

            if output.is_empty() {
                return Err(AppError::config_error(format!(
                    "Field '{}' must have entries in {}",
                    key,
                    path.display()
                )));
            }

            Ok(output)
        }
        Some(_) => Err(AppError::config_error(format!(
            "Field '{}' must be a list in {}",
            key,
            path.display()
        ))),
        None => Err(AppError::config_error(format!(
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
    use tempfile::tempdir;

    #[test]
    fn inspect_collects_counts_and_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let jules_path = root.join(".jules");
        let ws_dir = jules_path.join("workstreams").join("alpha");
        let exchange_dir = ws_dir.join("exchange");
        fs::create_dir_all(exchange_dir.join("events/pending")).unwrap();
        fs::create_dir_all(exchange_dir.join("events/decided")).unwrap();
        fs::create_dir_all(exchange_dir.join("issues/bugs")).unwrap();
        fs::create_dir_all(exchange_dir.join("issues/feats")).unwrap();

        fs::write(exchange_dir.join("events/pending/one.yml"), "id: abc123\n").unwrap();
        fs::write(exchange_dir.join("events/decided/two.yml"), "id: def456\n").unwrap();
        fs::write(
            exchange_dir.join("issues/bugs/bug.yml"),
            r#"
id: abc123
source_events:
  - abc123
requires_deep_analysis: false
"#,
        )
        .unwrap();

        fs::write(
            ws_dir.join("scheduled.toml"),
            r#"
version = 1
enabled = true
[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
[deciders]
roles = []
"#,
        )
        .unwrap();

        let output = inspect(
            &jules_path,
            WorkstreamInspectOptions {
                workstream: "alpha".to_string(),
                format: WorkstreamInspectFormat::Json,
            },
        )
        .unwrap();

        assert_eq!(output.workstream, "alpha");
        let pending = output.events.states.iter().find(|state| state.name == "pending").unwrap();
        assert_eq!(pending.count, 1);
        assert_eq!(output.events.pending_files.len(), 1);
        let bug_label = output.issues.labels.iter().find(|label| label.name == "bugs").unwrap();
        assert_eq!(bug_label.count, 1);
        assert_eq!(output.events.items.len(), 2);
        let pending_event =
            output.events.items.iter().find(|item| item.state == "pending").unwrap();
        assert_eq!(pending_event.id, "abc123");
        assert!(pending_event.path.ends_with("events/pending/one.yml"));

        assert_eq!(output.issues.items.len(), 1);
        let issue = &output.issues.items[0];
        assert_eq!(issue.id, "abc123");
        assert_eq!(issue.label, "bugs");
        assert!(!issue.requires_deep_analysis);
        assert_eq!(issue.source_events, vec!["abc123".to_string()]);
    }
}
