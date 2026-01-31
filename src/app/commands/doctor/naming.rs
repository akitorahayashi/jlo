use std::path::Path;

use super::diagnostics::Diagnostics;
use super::yaml::is_kebab_case;

pub fn naming_checks(
    jules_path: &Path,
    workstreams: &[String],
    issue_labels: &[String],
    event_states: &[String],
    diagnostics: &mut Diagnostics,
) {
    for workstream in workstreams {
        let ws_dir = jules_path.join("workstreams").join(workstream);
        let events_dir = ws_dir.join("events");
        for state in event_states {
            for entry in list_files(&events_dir.join(state)) {
                validate_filename(&entry, diagnostics, "event");
            }
        }

        let issues_dir = ws_dir.join("issues");
        for label in issue_labels {
            for entry in list_files(&issues_dir.join(label)) {
                validate_filename(&entry, diagnostics, "issue");
            }
        }
    }

    let observers_dir = jules_path.join("roles/observers");
    if observers_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&observers_dir)
    {
        for entry in entries.flatten() {
            let role_dir = entry.path();
            if role_dir.is_dir() {
                let feedback_dir = role_dir.join("feedbacks");
                for entry in list_files(&feedback_dir) {
                    validate_filename(&entry, diagnostics, "feedback");
                }
            }
        }
    }
}

fn validate_filename(path: &Path, diagnostics: &mut Diagnostics, kind: &str) {
    if path.file_name().and_then(|name| name.to_str()) == Some(".gitkeep") {
        return;
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
        diagnostics.push_error(path.display().to_string(), format!("{} file must be .yml", kind));
        return;
    }

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if !is_kebab_case(file_stem) {
        diagnostics.push_error(
            path.display().to_string(),
            format!("{} filename must be kebab-case", kind),
        );
    }
}

fn list_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
    }
    files
}
