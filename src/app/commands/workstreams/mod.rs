use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::{AppError, WorkstreamSchedule};

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
    pub roles: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EventSummary {
    pub states: Vec<EventStateSummary>,
    pub pending_files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EventStateSummary {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct IssueSummary {
    pub labels: Vec<IssueLabelSummary>,
}

#[derive(Debug, Serialize)]
pub struct IssueLabelSummary {
    pub name: String,
    pub count: usize,
    pub files: Vec<String>,
}

pub fn inspect(
    jules_path: &Path,
    options: WorkstreamInspectOptions,
) -> Result<WorkstreamInspectOutput, AppError> {
    if !jules_path.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

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
        observers: ScheduleLayerSummary { roles: schedule.observers.roles },
        deciders: ScheduleLayerSummary { roles: schedule.deciders.roles },
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
    let events_dir = ws_dir.join("events");
    if !events_dir.exists() {
        return Err(AppError::config_error(format!(
            "Missing events directory: {}",
            events_dir.display()
        )));
    }

    let mut states = Vec::new();
    let mut pending_files = Vec::new();

    let mut state_dirs: Vec<PathBuf> = fs::read_dir(&events_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect();
    state_dirs.sort();

    for state_dir in state_dirs {
        let state_name = state_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let files = list_yml_files(&state_dir)?;
        states.push(EventStateSummary { name: state_name.clone(), count: files.len() });

        if state_name == "pending" {
            pending_files = files.into_iter().map(|path| to_repo_relative(root, &path)).collect();
        }
    }

    Ok(EventSummary { states, pending_files })
}

fn summarize_issues(root: &Path, ws_dir: &Path) -> Result<IssueSummary, AppError> {
    let issues_dir = ws_dir.join("issues");
    if !issues_dir.exists() {
        return Err(AppError::config_error(format!(
            "Missing issues directory: {}",
            issues_dir.display()
        )));
    }

    let mut labels = Vec::new();
    let mut label_dirs: Vec<PathBuf> = fs::read_dir(&issues_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect();
    label_dirs.sort();

    for label_dir in label_dirs {
        let label_name = label_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let files = list_yml_files(&label_dir)?;
        let rel_files =
            files.into_iter().map(|path| to_repo_relative(root, &path)).collect::<Vec<_>>();
        labels.push(IssueLabelSummary {
            name: label_name,
            count: rel_files.len(),
            files: rel_files,
        });
    }

    Ok(IssueSummary { labels })
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

fn load_schedule(jules_path: &Path, workstream: &str) -> Result<WorkstreamSchedule, AppError> {
    let path = jules_path.join("workstreams").join(workstream).join("scheduled.toml");

    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::ScheduleConfigMissing(path.display().to_string())
        } else {
            AppError::config_error(format!("Failed to read {}: {}", path.display(), err))
        }
    })?;
    WorkstreamSchedule::parse_toml(&content).map_err(AppError::ScheduleConfigInvalid)
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
        fs::create_dir_all(ws_dir.join("events/pending")).unwrap();
        fs::create_dir_all(ws_dir.join("events/decided")).unwrap();
        fs::create_dir_all(ws_dir.join("issues/bugs")).unwrap();
        fs::create_dir_all(ws_dir.join("issues/feats")).unwrap();

        fs::write(ws_dir.join("events/pending/one.yml"), "id: a").unwrap();
        fs::write(ws_dir.join("events/decided/two.yml"), "id: b").unwrap();
        fs::write(ws_dir.join("issues/bugs/bug.yml"), "id: c").unwrap();

        fs::write(
            ws_dir.join("scheduled.toml"),
            r#"
version = 1
enabled = true
[observers]
roles = ["taxonomy"]
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
    }
}
