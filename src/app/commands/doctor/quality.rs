use std::path::Path;

use super::diagnostics::Diagnostics;
use super::yaml::{read_yaml_files, read_yaml_string, read_yaml_strings};

const MIN_STATEMENT_LEN: usize = 20;
const MIN_PROBLEM_LEN: usize = 20;
const MIN_IMPACT_LEN: usize = 20;
const MIN_DESIRED_OUTCOME_LEN: usize = 20;
const MIN_ACCEPTANCE_CRITERIA_LEN: usize = 8;

pub fn quality_checks(
    jules_path: &Path,
    issue_labels: &[String],
    event_states: &[String],
    diagnostics: &mut Diagnostics,
) {
    let exchange_dir = jules_path.join("exchange");
    let events_dir = exchange_dir.join("events");
    for state in event_states {
        for entry in read_yaml_files(&events_dir.join(state), diagnostics) {
            if let Some(statement) = read_yaml_string(&entry, "statement", diagnostics)
                && statement.trim().len() < MIN_STATEMENT_LEN
            {
                diagnostics
                    .push_warning(entry.display().to_string(), "statement appears too short");
            }
        }
    }

    let issues_dir = exchange_dir.join("issues");
    for label in issue_labels {
        for entry in read_yaml_files(&issues_dir.join(label), diagnostics) {
            if let Some(problem) = read_yaml_string(&entry, "problem", diagnostics)
                && problem.trim().len() < MIN_PROBLEM_LEN
            {
                diagnostics.push_warning(entry.display().to_string(), "problem appears too short");
            }
            if let Some(impact) = read_yaml_string(&entry, "impact", diagnostics)
                && impact.trim().len() < MIN_IMPACT_LEN
            {
                diagnostics.push_warning(entry.display().to_string(), "impact appears too short");
            }
            if let Some(desired) = read_yaml_string(&entry, "desired_outcome", diagnostics)
                && desired.trim().len() < MIN_DESIRED_OUTCOME_LEN
            {
                diagnostics
                    .push_warning(entry.display().to_string(), "desired_outcome appears too short");
            }

            if let Some(criteria) = read_yaml_strings(&entry, "acceptance_criteria", diagnostics) {
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

            if let Some(criteria) = read_yaml_strings(&entry, "verification_criteria", diagnostics)
            {
                for criterion in criteria {
                    let lowered = criterion.to_lowercase();
                    if criterion.contains('<')
                        || criterion.contains('>')
                        || lowered.contains("todo")
                        || lowered.contains("tbd")
                    {
                        diagnostics.push_warning(
                            entry.display().to_string(),
                            "verification_criteria entry looks incomplete",
                        );
                        break;
                    }
                }
            }
        }
    }
}
