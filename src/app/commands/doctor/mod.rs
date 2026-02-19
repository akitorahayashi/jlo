mod diagnostics;
mod naming;
mod quality;
mod schemas;
mod semantic;
mod structure;
mod yaml;

use std::path::Path;

use crate::adapters::catalogs::scaffold_assets::{
    list_event_states, list_issue_labels, read_enum_values,
};
use crate::domain::AppError;

#[allow(unused_imports)]
pub use diagnostics::{Diagnostic, Diagnostics, Severity};

#[derive(Debug, Clone, Default)]
pub struct DoctorOptions {
    pub strict: bool,
}

#[derive(Debug, Clone)]
pub struct DoctorOutcome {
    pub errors: usize,
    pub warnings: usize,
    pub exit_code: i32,
}

pub fn execute(jules_path: &Path, options: DoctorOptions) -> Result<DoctorOutcome, AppError> {
    if !jules_path.exists() {
        return Err(AppError::JulesNotFound);
    }

    let root = jules_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let issue_labels = list_issue_labels()?;
    let event_states = list_event_states()?;
    let event_confidence = read_enum_values(".jules/schemas/observers/event.yml", "confidence")?;
    let issue_priorities = read_enum_values(".jules/schemas/decider/requirements.yml", "priority")?;

    let mut diagnostics = Diagnostics::default();

    let _run_config = structure::read_control_plane_config(&root, &mut diagnostics)?;

    structure::structural_checks(
        structure::StructuralInputs { jules_path, root: &root, event_states: &event_states },
        &mut diagnostics,
    );

    schemas::schema_checks(
        schemas::SchemaInputs {
            jules_path,
            root: &root,
            issue_labels: &issue_labels,
            event_states: &event_states,
            event_confidence: &event_confidence,
            issue_priorities: &issue_priorities,
        },
        &mut diagnostics,
    );

    naming::naming_checks(jules_path, &event_states, &mut diagnostics);

    let semantic_context = semantic::semantic_context(jules_path, &mut diagnostics);
    semantic::semantic_checks(jules_path, &semantic_context, &mut diagnostics);

    quality::quality_checks(jules_path, &event_states, &mut diagnostics);

    diagnostics.emit();

    let errors = diagnostics.error_count();
    let warnings = diagnostics.warning_count();
    let exit_code = if errors > 0 {
        1
    } else if warnings > 0 && options.strict {
        2
    } else {
        0
    };

    if errors == 0 && warnings == 0 {
        println!("All checks passed.");
    } else if errors == 0 && !options.strict {
        eprintln!("Check completed with {} warning(s).", warnings);
    } else {
        eprintln!("Check failed: {} error(s), {} warning(s) found.", errors, warnings);
    }

    Ok(DoctorOutcome { errors, warnings, exit_code })
}
