use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, Utc};

use crate::domain::{AppError, Layer};
use crate::services::workstream_schedule_filesystem::load_schedule;

use super::diagnostics::Diagnostics;
use super::yaml::{read_yaml_bool, read_yaml_files, read_yaml_string, read_yaml_strings};

const STALE_DEEP_ANALYSIS_THRESHOLD_DAYS: i64 = 7;

#[derive(Debug, Default)]
pub struct SemanticContext {
    decided_events: HashMap<String, PathBuf>,
    event_issue_map: HashMap<String, String>,
    issues: HashMap<String, PathBuf>,
    issue_sources: HashMap<String, Vec<String>>,
}

pub fn semantic_context(
    jules_path: &Path,
    workstreams: &[String],
    issue_labels: &[String],
    diagnostics: &mut Diagnostics,
) -> SemanticContext {
    let mut context = SemanticContext::default();

    for workstream in workstreams {
        let ws_dir = jules_path.join("workstreams").join(workstream);
        let exchange_dir = ws_dir.join("exchange");
        let decided_dir = exchange_dir.join("events/decided");
        for entry in read_yaml_files(&decided_dir, diagnostics) {
            if let Some(id) = read_yaml_string(&entry, "id", diagnostics) {
                context.decided_events.insert(id.clone(), entry.clone());
                if let Some(issue_id) = read_yaml_string(&entry, "issue_id", diagnostics)
                    && !issue_id.is_empty()
                {
                    context.event_issue_map.insert(id, issue_id);
                }
            }
        }

        let issues_dir = exchange_dir.join("issues");
        for label in issue_labels {
            for entry in read_yaml_files(&issues_dir.join(label), diagnostics) {
                if let Some(id) = read_yaml_string(&entry, "id", diagnostics) {
                    context.issues.insert(id.clone(), entry.clone());
                    if let Some(source_events) =
                        read_yaml_strings(&entry, "source_events", diagnostics)
                    {
                        context.issue_sources.insert(id, source_events);
                    }
                }
            }
        }
    }

    context
}

pub fn semantic_checks(
    jules_path: &Path,
    workstreams: &[String],
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

    // Workstream-prompt relationship is now managed through scheduled.toml
    // Roles are generic and assigned to workstreams via the schedule, not the role.yml

    // Collect existing roles from filesystem for each layer
    // With the new scaffold structure, roles are under .jules/roles/<layer>/roles/<role>/
    let roles_dir = jules_path.join("roles");
    let mut existing_roles: HashMap<Layer, HashSet<String>> = HashMap::new();
    for layer in [Layer::Observers, Layer::Deciders] {
        let roles_container = roles_dir.join(layer.dir_name()).join("roles");
        if roles_container.exists() {
            let mut role_set = HashSet::new();
            match std::fs::read_dir(&roles_container) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            role_set.insert(name);
                        }
                    }
                }
                Err(err) => {
                    diagnostics.push_error(
                        roles_container.display().to_string(),
                        format!("Failed to read directory: {}", err),
                    );
                }
            }
            existing_roles.insert(layer, role_set);
        }
    }

    let mut scheduled_roles: HashMap<Layer, HashSet<String>> = HashMap::new();
    for workstream in workstreams {
        match load_schedule(jules_path, workstream) {
            Ok(schedule) => {
                for role in schedule.observers.roles {
                    scheduled_roles.entry(Layer::Observers).or_default().insert(role.name.as_str().to_string());
                    // Validate role exists in filesystem
                    if !existing_roles
                        .get(&Layer::Observers)
                        .is_some_and(|roles| roles.contains(role.name.as_str()))
                    {
                        diagnostics.push_error(
                            role.name.as_str().to_string(),
                            "Observer role listed in scheduled.toml but missing from filesystem",
                        );
                    }
                }

                for role in schedule.deciders.roles {
                    scheduled_roles.entry(Layer::Deciders).or_default().insert(role.name.as_str().to_string());
                    // Validate role exists in filesystem
                    if !existing_roles
                        .get(&Layer::Deciders)
                        .is_some_and(|roles| roles.contains(role.name.as_str()))
                    {
                        diagnostics.push_error(
                            role.name.as_str().to_string(),
                            "Decider role listed in scheduled.toml but missing from filesystem",
                        );
                    }
                }
            }
            Err(AppError::ScheduleConfigMissing(_)) => {
                // structural checks handle missing scheduled.toml (including --fix)
            }
            Err(AppError::Schedule(err)) => {
                diagnostics
                    .push_error(workstream.clone(), format!("Invalid scheduled.toml: {}", err));
            }
            Err(err) => {
                diagnostics.push_error(workstream.clone(), err.to_string());
            }
        }
    }

    // Check for roles that exist but aren't scheduled in any workstream (dangling roles)
    for (layer, roles) in &existing_roles {
        let scheduled = scheduled_roles.get(layer).cloned().unwrap_or_default();
        for role in roles {
            if !scheduled.contains(role) {
                diagnostics.push_warning(
                    role.clone(),
                    "Role not listed in any scheduled.toml (dangling role)",
                );
            }
        }
    }

    for path in context.issues.values() {
        if let Some(requires) = read_yaml_bool(path, "requires_deep_analysis", diagnostics)
            && requires
        {
            match read_yaml_string(path, "deep_analysis_reason", diagnostics) {
                Some(reason) if !reason.trim().is_empty() => {}
                _ => {
                    diagnostics.push_error(
                        path.display().to_string(),
                        "requires_deep_analysis true without deep_analysis_reason",
                    );
                }
            }

            if let Some(date) = read_yaml_string(path, "created_at", diagnostics)
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
}
