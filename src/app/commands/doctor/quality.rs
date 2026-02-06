use std::path::Path;

use crate::ports::WorkspaceStore;

use super::diagnostics::Diagnostics;
use super::yaml::{read_yaml_files, read_yaml_string, read_yaml_strings};

const MIN_STATEMENT_LEN: usize = 20;
const MIN_PROBLEM_LEN: usize = 20;
const MIN_IMPACT_LEN: usize = 20;
const MIN_DESIRED_OUTCOME_LEN: usize = 20;
const MIN_ACCEPTANCE_CRITERIA_LEN: usize = 8;

pub fn quality_checks(
    store: &impl WorkspaceStore,
    jules_path: &Path,
    workstreams: &[String],
    issue_labels: &[String],
    event_states: &[String],
    diagnostics: &mut Diagnostics,
) {
    for workstream in workstreams {
        let ws_dir = jules_path.join("workstreams").join(workstream);
        let exchange_dir = ws_dir.join("exchange");
        let events_dir = exchange_dir.join("events");
        for state in event_states {
            for entry in read_yaml_files(store, &events_dir.join(state), diagnostics) {
                if let Some(statement) = read_yaml_string(store, &entry, "statement", diagnostics)
                    && statement.trim().len() < MIN_STATEMENT_LEN
                {
                    diagnostics
                        .push_warning(entry.display().to_string(), "statement appears too short");
                }
            }
        }

        let issues_dir = exchange_dir.join("issues");
        for label in issue_labels {
            for entry in read_yaml_files(store, &issues_dir.join(label), diagnostics) {
                if let Some(problem) = read_yaml_string(store, &entry, "problem", diagnostics)
                    && problem.trim().len() < MIN_PROBLEM_LEN
                {
                    diagnostics
                        .push_warning(entry.display().to_string(), "problem appears too short");
                }
                if let Some(impact) = read_yaml_string(store, &entry, "impact", diagnostics)
                    && impact.trim().len() < MIN_IMPACT_LEN
                {
                    diagnostics
                        .push_warning(entry.display().to_string(), "impact appears too short");
                }
                if let Some(desired) = read_yaml_string(store, &entry, "desired_outcome", diagnostics)
                    && desired.trim().len() < MIN_DESIRED_OUTCOME_LEN
                {
                    diagnostics.push_warning(
                        entry.display().to_string(),
                        "desired_outcome appears too short",
                    );
                }

                if let Some(criteria) =
                    read_yaml_strings(store, &entry, "acceptance_criteria", diagnostics)
                {
                    for item in criteria {
                        if item.trim().len() < MIN_ACCEPTANCE_CRITERIA_LEN {
                            diagnostics.push_warning(
                                entry.display().to_string(),
                                "acceptance_criteria entry appears too short",
                            );
                            break;
                        }
                    }
                }

                if let Some(commands) =
                    read_yaml_strings(store, &entry, "verification_commands", diagnostics)
                {
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
