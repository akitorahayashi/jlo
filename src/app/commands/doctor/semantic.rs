use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, Utc};

use crate::app::config::load_schedule;
use crate::domain::config::schedule::ScheduleLayer;
use crate::domain::{AppError, Layer};

use super::diagnostics::Diagnostics;
use super::yaml::{read_yaml_bool, read_yaml_files, read_yaml_string, read_yaml_strings};

const STALE_IMPLEMENTATION_PENDING_THRESHOLD_DAYS: i64 = 7;

#[derive(Debug, Default)]
pub struct SemanticContext {
    decided_events: HashMap<String, PathBuf>,
    event_requirement_map: HashMap<String, String>,
    requirements: HashMap<String, PathBuf>,
    requirement_sources: HashMap<String, Vec<String>>,
}

pub fn semantic_context(jules_path: &Path, diagnostics: &mut Diagnostics) -> SemanticContext {
    let mut context = SemanticContext::default();

    let decided_dir = crate::domain::exchange::events::paths::events_decided_dir(jules_path);
    for entry in read_yaml_files(&decided_dir, diagnostics) {
        if let Some(id) = read_yaml_string(&entry, "id", diagnostics) {
            context.decided_events.insert(id.clone(), entry.clone());
            if let Some(requirement_id) = read_yaml_string(&entry, "requirement_id", diagnostics)
                && !requirement_id.is_empty()
            {
                context.event_requirement_map.insert(id, requirement_id);
            }
        }
    }

    let requirements_dir =
        crate::domain::exchange::requirements::paths::requirements_dir(jules_path);
    for entry in read_yaml_files(&requirements_dir, diagnostics) {
        if let Some(id) = read_yaml_string(&entry, "id", diagnostics) {
            context.requirements.insert(id.clone(), entry.clone());
            if let Some(source_events) = read_yaml_strings(&entry, "source_events", diagnostics) {
                context.requirement_sources.insert(id, source_events);
            }
        }
    }

    context
}

pub fn semantic_checks(
    jules_path: &Path,
    context: &SemanticContext,
    diagnostics: &mut Diagnostics,
) {
    let event_source_index = build_event_source_index(context);

    for (event_id, requirement_id) in &context.event_requirement_map {
        if !context.requirements.contains_key(requirement_id)
            && let Some(path) = context.decided_events.get(event_id)
        {
            diagnostics.push_error(
                path.display().to_string(),
                format!("requirement_id '{}' does not exist", requirement_id),
            );
        }
    }

    for (requirement_id, sources) in &context.requirement_sources {
        for source in sources {
            if !context.decided_events.contains_key(source)
                && let Some(path) = context.requirements.get(requirement_id)
            {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!("source_events refers to missing event '{}'", source),
                );
            }
        }
    }

    for (event_id, requirement_ids) in &event_source_index {
        if requirement_ids.len() > 1 {
            let owners = requirement_ids.join(", ");
            for requirement_id in requirement_ids {
                if let Some(path) = context.requirements.get(requirement_id) {
                    diagnostics.push_error(
                        path.display().to_string(),
                        format!(
                            "event '{}' is referenced by multiple requirements in source_events: {}",
                            event_id, owners
                        ),
                    );
                }
            }
        }
    }

    for (event_id, requirement_id) in &context.event_requirement_map {
        if let Some(owners) = event_source_index.get(event_id) {
            if !owners.iter().any(|owner| owner == requirement_id)
                && let Some(path) = context.decided_events.get(event_id)
            {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!(
                        "event '{}' requirement_id '{}' does not match requirement source owner(s): {}",
                        event_id,
                        requirement_id,
                        owners.join(", ")
                    ),
                );
            }
        } else if let Some(path) = context.decided_events.get(event_id) {
            diagnostics.push_error(
                path.display().to_string(),
                format!(
                    "event '{}' has requirement_id '{}' but is not referenced by any requirement source_events",
                    event_id, requirement_id
                ),
            );
        }
    }

    for (requirement_id, sources) in &context.requirement_sources {
        for source in sources {
            if let Some(event_requirement_id) = context.event_requirement_map.get(source)
                && event_requirement_id != requirement_id
                && let Some(path) = context.requirements.get(requirement_id)
            {
                diagnostics.push_error(
                    path.display().to_string(),
                    format!(
                        "source event '{}' belongs to requirement '{}' via event.requirement_id, but was found in requirement '{}'",
                        source, event_requirement_id, requirement_id
                    ),
                );
            }
        }
    }

    // Exchange-prompt relationship is managed through config schedule sections.
    // Roles are generic and assigned to the exchange via the schedule, not the role.yml

    // Collect existing roles from filesystem for each layer
    // Roles are user-defined and live under .jlo/roles/<layer>/<role>/
    let root = match jules_path.parent() {
        Some(p) => p,
        None => {
            diagnostics.push_error(
                jules_path.display().to_string(),
                "Could not determine parent directory of .jules path".to_string(),
            );
            return;
        }
    };
    let mut existing_roles: HashMap<Layer, HashSet<String>> = HashMap::new();
    // Only validate multi-role layers that are scheduled (Observers, Innovators)
    for layer in [Layer::Observers, Layer::Innovators] {
        let layer_dir = crate::domain::roles::paths::layer_dir(root, layer);
        if layer_dir.exists() {
            let mut role_set = HashSet::new();
            match std::fs::read_dir(&layer_dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if path.join("role.yml").exists() {
                                role_set.insert(name);
                            }
                        }
                    }
                }
                Err(err) => {
                    diagnostics.push_error(
                        layer_dir.display().to_string(),
                        format!("Failed to read directory: {}", err),
                    );
                }
            }
            existing_roles.insert(layer, role_set);
        }
    }

    let store = crate::adapters::local_repository::LocalRepositoryAdapter::new(root.to_path_buf());

    match load_schedule(&store) {
        Ok(schedule) => {
            validate_scheduled_layer(
                Layer::Observers,
                &schedule.observers,
                &existing_roles,
                diagnostics,
            );
            if let Some(ref innovators) = schedule.innovators {
                validate_scheduled_layer(
                    Layer::Innovators,
                    innovators,
                    &existing_roles,
                    diagnostics,
                );
            }
        }
        Err(AppError::ControlPlaneConfigMissing) => {
            // structural checks handle missing config.toml
        }
        Err(AppError::Schedule(err)) => {
            diagnostics.push_error(
                "config.toml".to_string(),
                format!("Invalid schedule in .jlo/config.toml: {}", err),
            );
        }
        Err(err) => {
            diagnostics.push_error("config.toml".to_string(), err.to_string());
        }
    }

    for path in context.requirements.values() {
        if let Some(implementation_ready) =
            read_yaml_bool(path, "implementation_ready", diagnostics)
            && !implementation_ready
        {
            match read_yaml_string(path, "planner_request_reason", diagnostics) {
                Some(reason) if !reason.trim().is_empty() => {}
                _ => {
                    diagnostics.push_error(
                        path.display().to_string(),
                        "implementation_ready false without planner_request_reason",
                    );
                }
            }

            if let Some(date) = read_yaml_string(path, "created_at", diagnostics)
                && let Ok(parsed) = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            {
                let days = (Utc::now().date_naive() - parsed).num_days();
                if days > STALE_IMPLEMENTATION_PENDING_THRESHOLD_DAYS {
                    diagnostics.push_warning(
                        path.display().to_string(),
                        format!("implementation_ready false for {} days", days),
                    );
                }
            }
        }
    }
}

fn validate_scheduled_layer(
    layer: Layer,
    schedule_layer: &ScheduleLayer,
    existing_roles: &HashMap<Layer, HashSet<String>>,
    diagnostics: &mut Diagnostics,
) {
    for role in &schedule_layer.roles {
        let role_name = role.name.as_str();
        let exists_as_custom =
            existing_roles.get(&layer).is_some_and(|roles| roles.contains(role_name));

        if !exists_as_custom {
            diagnostics.push_error(
                role_name.to_string(),
                format!(
                    "{} role listed in .jlo/config.toml schedule but missing from .jlo/roles/{}/<role>/role.yml",
                    layer.display_name()
                    ,
                    layer.dir_name()
                ),
            );
        }
    }
}

fn build_event_source_index(context: &SemanticContext) -> HashMap<String, Vec<String>> {
    let mut index: HashMap<String, Vec<String>> = HashMap::new();
    for (requirement_id, sources) in &context.requirement_sources {
        for source in sources {
            index.entry(source.clone()).or_default().push(requirement_id.clone());
        }
    }
    for owners in index.values_mut() {
        owners.sort();
        owners.dedup();
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_minimal_workspace(root: &Path) {
        fs::create_dir_all(root.join(".jules/exchange/events/decided"))
            .expect("create decided dir");
        fs::create_dir_all(root.join(".jules/exchange/requirements"))
            .expect("create requirements dir");
        fs::create_dir_all(root.join(".jlo/roles/observers/taxonomy"))
            .expect("create observer role dir");
        fs::write(
            root.join(".jlo/roles/observers/taxonomy/role.yml"),
            "role: taxonomy\nlayer: observers\nprofile:\n  focus: test\n",
        )
        .expect("write observer role");
        fs::write(
            root.join(".jlo/config.toml"),
            r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[observers]
roles = [
  { name = "taxonomy", enabled = true },
]
"#,
        )
        .expect("write config");
    }

    #[test]
    fn semantic_checks_reject_event_referenced_by_multiple_requirements() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        write_minimal_workspace(root);

        fs::write(
            root.join(".jules/exchange/events/decided/event-a.yml"),
            "id: abc123\nrequirement_id: req111\n",
        )
        .expect("write event");
        fs::write(
            root.join(".jules/exchange/requirements/req-one.yml"),
            "id: req111\nsource_events:\n  - abc123\n",
        )
        .expect("write requirement one");
        fs::write(
            root.join(".jules/exchange/requirements/req-two.yml"),
            "id: req222\nsource_events:\n  - abc123\n",
        )
        .expect("write requirement two");

        let mut diagnostics = Diagnostics::default();
        let context = semantic_context(&root.join(".jules"), &mut diagnostics);
        semantic_checks(&root.join(".jules"), &context, &mut diagnostics);

        assert!(diagnostics.errors().iter().any(|diag| {
            diag.message.contains("referenced by multiple requirements in source_events")
        }));
    }

    #[test]
    fn semantic_checks_reject_requirement_id_source_owner_mismatch() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        write_minimal_workspace(root);

        fs::write(
            root.join(".jules/exchange/events/decided/event-a.yml"),
            "id: abc123\nrequirement_id: req111\n",
        )
        .expect("write event a");
        fs::write(
            root.join(".jules/exchange/events/decided/event-b.yml"),
            "id: def456\nrequirement_id: req111\n",
        )
        .expect("write event b");
        fs::write(
            root.join(".jules/exchange/requirements/req-one.yml"),
            "id: req111\nsource_events:\n  - def456\n",
        )
        .expect("write requirement one");
        fs::write(
            root.join(".jules/exchange/requirements/req-two.yml"),
            "id: req222\nsource_events:\n  - abc123\n",
        )
        .expect("write requirement two");

        let mut diagnostics = Diagnostics::default();
        let context = semantic_context(&root.join(".jules"), &mut diagnostics);
        semantic_checks(&root.join(".jules"), &context, &mut diagnostics);

        assert!(
            diagnostics
                .errors()
                .iter()
                .any(|diag| { diag.message.contains("does not match requirement source owner") })
        );
        assert!(
            diagnostics
                .errors()
                .iter()
                .any(|diag| { diag.message.contains("belongs to requirement 'req111'") })
        );
    }
}
