use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, Utc};

use crate::domain::RunConfig;

use super::diagnostics::Diagnostics;
use super::structure::list_subdirs;
use super::yaml::{read_yaml_bool, read_yaml_files, read_yaml_string, read_yaml_strings};

const STALE_DEEP_ANALYSIS_THRESHOLD_DAYS: i64 = 7;

#[derive(Debug, Default)]
pub struct SemanticContext {
    decided_events: HashMap<String, PathBuf>,
    event_issue_map: HashMap<String, String>,
    issues: HashMap<String, PathBuf>,
    issue_sources: HashMap<String, Vec<String>>,
    index_entries: HashMap<String, HashSet<String>>,
    index_duplicates: HashMap<String, Vec<String>>,
}

pub fn semantic_context(
    jules_path: &Path,
    workstreams: &[String],
    issue_labels: &[String],
) -> SemanticContext {
    let mut context = SemanticContext::default();

    for workstream in workstreams {
        let ws_dir = jules_path.join("workstreams").join(workstream);
        let decided_dir = ws_dir.join("events/decided");
        for entry in read_yaml_files(&decided_dir) {
            if let Some(id) = read_yaml_string(&entry, "id") {
                context.decided_events.insert(id.clone(), entry.clone());
                if let Some(issue_id) = read_yaml_string(&entry, "issue_id")
                    && !issue_id.is_empty()
                {
                    context.event_issue_map.insert(id, issue_id);
                }
            }
        }

        let issues_dir = ws_dir.join("issues");
        for label in issue_labels {
            for entry in read_yaml_files(&issues_dir.join(label)) {
                if let Some(id) = read_yaml_string(&entry, "id") {
                    context.issues.insert(id.clone(), entry.clone());
                    if let Some(source_events) = read_yaml_strings(&entry, "source_events") {
                        context.issue_sources.insert(id, source_events);
                    }
                }
            }
        }

        let index_path = issues_dir.join("index.md");
        if let Ok(content) = fs::read_to_string(&index_path) {
            let parsed = parse_index_entries(&content);
            if !parsed.entries.is_empty() {
                context.index_entries.insert(workstream.clone(), parsed.entries);
            }
            if !parsed.duplicates.is_empty() {
                context.index_duplicates.insert(workstream.clone(), parsed.duplicates);
            }
        }
    }

    context
}

pub fn semantic_checks(
    jules_path: &Path,
    workstreams: &[String],
    prompt_workstreams: &HashSet<String>,
    run_config: &RunConfig,
    context: &SemanticContext,
    diagnostics: &mut Diagnostics,
) {
    for (event_id, issue_id) in &context.event_issue_map {
        if !context.issues.contains_key(issue_id)
            && let Some(path) = context.decided_events.get(event_id)
        {
            diagnostics.push_error(
                path.display().to_string(),
                format!("issue_id '{}' does not exist", issue_id),
            );
        }
    }

    for (issue_id, sources) in &context.issue_sources {
        for source in sources {
            if !context.decided_events.contains_key(source)
                && let Some(path) = context.issues.get(issue_id)
            {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("source_events refers to missing event '{}'", source),
                );
            }
        }
    }

    for workstream in workstreams {
        if let Some(entries) = context.index_entries.get(workstream) {
            let ws_dir = jules_path.join("workstreams").join(workstream).join("issues");
            let mut files = HashSet::new();
            for entry in walk_issue_files(&ws_dir) {
                if let Ok(rel) = entry.strip_prefix(&ws_dir) {
                    files.insert(rel.to_string_lossy().to_string());
                }
            }

            for entry in entries {
                if !files.contains(entry) {
                    diagnostics.push_error(
                        ws_dir.join("index.md").display().to_string(),
                        format!("index entry '{}' has no matching file", entry),
                    );
                }
            }

            for file in files {
                if !entries.contains(&file) {
                    diagnostics.push_error(
                        ws_dir.join("index.md").display().to_string(),
                        format!("issue file '{}' not listed in index", file),
                    );
                }
            }
        }

        if let Some(duplicates) = context.index_duplicates.get(workstream) {
            let index_path =
                jules_path.join("workstreams").join(workstream).join("issues/index.md");
            for entry in duplicates {
                diagnostics.push_error(
                    index_path.display().to_string(),
                    format!("duplicate index entry '{}'", entry),
                );
            }
        }
    }

    for workstream in prompt_workstreams {
        if !workstreams.contains(workstream) {
            diagnostics
                .push_error(workstream.clone(), "workstream referenced in prompt does not exist");
        }
    }

    for workstream in workstreams {
        if !prompt_workstreams.contains(workstream) {
            diagnostics
                .push_error(workstream.clone(), "workstream exists but no prompt references it");
        }
    }

    let observers_dir = jules_path.join("roles/observers");
    let deciders_dir = jules_path.join("roles/deciders");

    let observer_dirs: HashSet<String> = list_subdirs(&observers_dir)
        .iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()).map(|s| s.to_string()))
        .collect();

    let decider_dirs: HashSet<String> = list_subdirs(&deciders_dir)
        .iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()).map(|s| s.to_string()))
        .collect();

    for role in &run_config.agents.observers {
        if !observer_dirs.contains(role) {
            diagnostics.push_error(role.clone(), "Observer role missing from filesystem");
        }
    }

    for role in &run_config.agents.deciders {
        if !decider_dirs.contains(role) {
            diagnostics.push_error(role.clone(), "Decider role missing from filesystem");
        }
    }

    for role in observer_dirs {
        if !run_config.agents.observers.contains(&role) {
            diagnostics.push_error(role.clone(), "Observer role not listed in config.toml");
        }
    }

    for role in decider_dirs {
        if !run_config.agents.deciders.contains(&role) {
            diagnostics.push_error(role.clone(), "Decider role not listed in config.toml");
        }
    }

    for path in context.issues.values() {
        if let Some(requires) = read_yaml_bool(path, "requires_deep_analysis")
            && requires
        {
            match read_yaml_string(path, "deep_analysis_reason") {
                Some(reason) if !reason.trim().is_empty() => {}
                _ => {
                    diagnostics.push_error(
                        path.display().to_string(),
                        "requires_deep_analysis true without deep_analysis_reason",
                    );
                }
            }
        }
    }
    for path in context.issues.values() {
        if let Some(requires) = read_yaml_bool(path, "requires_deep_analysis")
            && requires
            && let Some(date) = read_yaml_string(path, "created_at")
            && let Ok(parsed) = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        {
            let days = (Utc::now().date_naive() - parsed).num_days();
            if days > STALE_DEEP_ANALYSIS_THRESHOLD_DAYS {
                diagnostics.push_warning(
                    path.display().to_string(),
                    format!("requires_deep_analysis true for {} days", days),
                );
            }
        }
    }
}

struct IndexParseResult {
    entries: HashSet<String>,
    duplicates: Vec<String>,
}

fn parse_index_entries(content: &str) -> IndexParseResult {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut in_comment = false;
    for line in content.lines() {
        if line.contains("<!--") {
            in_comment = true;
        }

        if !in_comment
            && let Some(start) = line.find("](./")
            && let Some(end) = line[start + 4..].find(')')
        {
            let slice = &line[start + 4..start + 4 + end];
            if slice.ends_with(".yml") {
                *counts.entry(slice.to_string()).or_insert(0) += 1;
            }
        }

        if line.contains("-->") {
            in_comment = false;
        }
    }

    let mut entries = HashSet::new();
    let mut duplicates = Vec::new();
    for (entry, count) in counts {
        if count > 1 {
            duplicates.push(entry.clone());
        }
        entries.insert(entry);
    }

    IndexParseResult { entries, duplicates }
}

fn walk_issue_files(issues_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(issues_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(read_yaml_files(&path));
            }
        }
    }
    files
}
