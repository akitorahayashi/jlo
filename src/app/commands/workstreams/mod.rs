use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::domain::{AppError, JULES_DIR};
use crate::ports::WorkspaceStore;
use crate::services::adapters::workstream_schedule_filesystem::load_schedule;

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
    workspace: &impl WorkspaceStore,
    options: WorkstreamInspectOptions,
) -> Result<WorkstreamInspectOutput, AppError> {
    let ws_dir = format!("{}/workstreams/{}", JULES_DIR, options.workstream);
    if !workspace.path_exists(&ws_dir) {
        return Err(AppError::Validation(format!("Workstream '{}' not found", options.workstream)));
    }

    let schedule = load_schedule(workspace, &options.workstream)?;
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

    let events = summarize_events(workspace, &ws_dir)?;
    let issues = summarize_issues(workspace, &ws_dir)?;

    Ok(WorkstreamInspectOutput {
        schema_version: 1,
        workstream: options.workstream,
        schedule: schedule_summary,
        events,
        issues,
    })
}

fn summarize_events(workspace: &impl WorkspaceStore, ws_dir: &str) -> Result<EventSummary, AppError> {
    let events_dir = format!("{}/exchange/events", ws_dir);
    if !workspace.path_exists(&events_dir) {
        return Err(AppError::Validation(format!(
            "Missing events directory: {}",
            events_dir
        )));
    }

    let mut states = Vec::new();
    let mut pending_files = Vec::new();
    let mut items = Vec::new();

    let state_dirs = workspace.list_dirs(&events_dir)?;

    for state_name in state_dirs {
        let state_dir = format!("{}/{}", events_dir, state_name);

        let files = list_yml_files(workspace, &state_dir)?;
        states.push(EventStateSummary { name: state_name.clone(), count: files.len() });

        if state_name == "pending" {
            pending_files = files.clone();
        }

        for path in &files {
            let item = read_event_item(workspace, path, &state_name)?;
            items.push(item);
        }
    }

    items.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(EventSummary { states, pending_files, items })
}

fn summarize_issues(workspace: &impl WorkspaceStore, ws_dir: &str) -> Result<IssueSummary, AppError> {
    let issues_dir = format!("{}/exchange/issues", ws_dir);
    if !workspace.path_exists(&issues_dir) {
        return Err(AppError::Validation(format!(
            "Missing issues directory: {}",
            issues_dir
        )));
    }

    let mut labels = Vec::new();
    let mut items = Vec::new();
    let label_dirs = workspace.list_dirs(&issues_dir)?;

    for label_name in label_dirs {
        let label_dir = format!("{}/{}", issues_dir, label_name);

        let files = list_yml_files(workspace, &label_dir)?;

        labels.push(IssueLabelSummary {
            name: label_name.clone(),
            count: files.len(),
            files: files.clone(),
        });

        for path in &files {
            let item = read_issue_item(workspace, path, &label_name)?;
            items.push(item);
        }
    }

    items.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(IssueSummary { labels, items })
}

fn list_yml_files(workspace: &impl WorkspaceStore, dir: &str) -> Result<Vec<String>, AppError> {
    let filenames = workspace.list_files(dir)?;
    let mut paths = Vec::new();
    for filename in filenames {
        if filename.ends_with(".yml") {
            paths.push(format!("{}/{}", dir, filename));
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_event_item(workspace: &impl WorkspaceStore, path: &str, state: &str) -> Result<EventItem, AppError> {
    let map = read_yaml_mapping(workspace, path)?;
    let id = read_required_id(&map, path, "id")?;

    Ok(EventItem { path: path.to_string(), state: state.to_string(), id })
}

fn read_issue_item(workspace: &impl WorkspaceStore, path: &str, label: &str) -> Result<IssueItem, AppError> {
    let map = read_yaml_mapping(workspace, path)?;
    let id = read_required_id(&map, path, "id")?;
    let requires_deep_analysis = read_required_bool(&map, path, "requires_deep_analysis")?;
    let source_events = read_required_string_list(&map, path, "source_events")?;

    Ok(IssueItem {
        path: path.to_string(),
        label: label.to_string(),
        requires_deep_analysis,
        id,
        source_events,
    })
}

fn read_yaml_mapping(workspace: &impl WorkspaceStore, path: &str) -> Result<Mapping, AppError> {
    let content = workspace.read_file(path)?;
    let value: Value = serde_yaml::from_str(&content).map_err(|err| AppError::ParseError {
        what: path.to_string(),
        details: err.to_string(),
    })?;

    match value {
        Value::Mapping(map) => Ok(map),
        _ => {
            Err(AppError::Validation(format!("YAML root must be a mapping in {}", path)))
        }
    }
}

fn read_required_string(map: &Mapping, path: &str, key: &str) -> Result<String, AppError> {
    match map.get(Value::String(key.to_string())) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        Some(Value::String(_)) => Err(AppError::Validation(format!(
            "Field '{}' must be non-empty in {}",
            key,
            path
        ))),
        Some(_) => Err(AppError::Validation(format!(
            "Field '{}' must be a string in {}",
            key,
            path
        ))),
        None => Err(AppError::Validation(format!(
            "Missing required field '{}' in {}",
            key,
            path
        ))),
    }
}

fn read_required_id(map: &Mapping, path: &str, key: &str) -> Result<String, AppError> {
    let value = read_required_string(map, path, key)?;
    if !is_valid_id(&value) {
        return Err(AppError::Validation(format!(
            "Field '{}' must be 6 lowercase alphanumeric chars in {}",
            key,
            path
        )));
    }
    Ok(value)
}

fn read_required_bool(map: &Mapping, path: &str, key: &str) -> Result<bool, AppError> {
    match map.get(Value::String(key.to_string())) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(AppError::Validation(format!(
            "Field '{}' must be a boolean in {}",
            key,
            path
        ))),
        None => Err(AppError::Validation(format!(
            "Missing required field '{}' in {}",
            key,
            path
        ))),
    }
}

fn read_required_string_list(
    map: &Mapping,
    path: &str,
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
                        path
                    ))),
                    _ => Err(AppError::Validation(format!(
                        "Field '{}' must contain strings in {}",
                        key,
                        path
                    ))),
                })
                .collect();

            let output = output?;

            if output.is_empty() {
                return Err(AppError::Validation(format!(
                    "Field '{}' must have entries in {}",
                    key,
                    path
                )));
            }

            Ok(output)
        }
        Some(_) => Err(AppError::Validation(format!(
            "Field '{}' must be a list in {}",
            key,
            path
        ))),
        None => Err(AppError::Validation(format!(
            "Missing required field '{}' in {}",
            key,
            path
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
    use crate::services::adapters::workspace_filesystem::FilesystemWorkspaceStore;
    use std::fs; // Used to setup test environment via fs

    #[test]
    fn inspect_collects_counts_and_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let workspace = FilesystemWorkspaceStore::new(root.to_path_buf());
        workspace.create_structure(&[]).unwrap();

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
            &workspace,
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
