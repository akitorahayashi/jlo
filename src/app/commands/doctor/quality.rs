use std::path::Path;

use super::diagnostics::Diagnostics;
use super::yaml::{read_yaml_files, read_yaml_string, read_yaml_strings};

pub fn quality_checks(
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
            for entry in read_yaml_files(&events_dir.join(state)) {
                if let Some(statement) = read_yaml_string(&entry, "statement")
                    && statement.trim().len() < 20
                {
                    diagnostics
                        .push_warning(entry.display().to_string(), "statement appears too short");
                }
            }
        }

        let issues_dir = ws_dir.join("issues");
        for label in issue_labels {
            for entry in read_yaml_files(&issues_dir.join(label)) {
                if let Some(problem) = read_yaml_string(&entry, "problem")
                    && problem.trim().len() < 20
                {
                    diagnostics
                        .push_warning(entry.display().to_string(), "problem appears too short");
                }
                if let Some(impact) = read_yaml_string(&entry, "impact")
                    && impact.trim().len() < 20
                {
                    diagnostics
                        .push_warning(entry.display().to_string(), "impact appears too short");
                }
                if let Some(desired) = read_yaml_string(&entry, "desired_outcome")
                    && desired.trim().len() < 20
                {
                    diagnostics.push_warning(
                        entry.display().to_string(),
                        "desired_outcome appears too short",
                    );
                }

                if let Some(criteria) = read_yaml_strings(&entry, "acceptance_criteria") {
                    for item in criteria {
                        if item.trim().len() < 8 {
                            diagnostics.push_warning(
                                entry.display().to_string(),
                                "acceptance_criteria entry appears too short",
                            );
                            break;
                        }
                    }
                }

                if let Some(commands) = read_yaml_strings(&entry, "verification_commands") {
                    for command in commands {
                        let lowered = command.to_lowercase();
                        if command.contains('<')
                            || command.contains('>')
                            || lowered.contains("todo")
                            || lowered.contains("tbd")
                        {
                            diagnostics.push_warning(
                                entry.display().to_string(),
                                "verification_commands entry looks non-executable",
                            );
                            break;
                        }
                    }
                }
            }
        }
    }
}
