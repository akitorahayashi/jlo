mod diagnostics;
mod naming;
mod quality;
mod schema;
mod semantic;
mod structure;
mod yaml;

use std::path::Path;

use crate::domain::AppError;
use crate::ports::WorkspaceStore;
use crate::services::assets::scaffold_assets::{
    list_event_states, list_issue_labels, read_enum_values,
};

#[allow(unused_imports)]
pub use diagnostics::{Diagnostic, Diagnostics, Severity};

#[derive(Debug, Clone, Default)]
pub struct DoctorOptions {
    pub fix: bool,
    pub strict: bool,
    pub workstream: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DoctorOutcome {
    pub errors: usize,
    pub warnings: usize,
    pub exit_code: i32,
}

pub fn execute(store: &impl WorkspaceStore, options: DoctorOptions) -> Result<DoctorOutcome, AppError> {
    if !store.exists() {
        return Err(AppError::WorkspaceNotFound);
    }

    let jules_path = store.jules_path();
    let root = jules_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    let issue_labels = list_issue_labels()?;
    let event_states = list_event_states()?;
    let event_confidence =
        read_enum_values(".jules/roles/observers/schemas/event.yml", "confidence")?;
    let issue_priorities = read_enum_values(".jules/roles/deciders/schemas/issue.yml", "priority")?;

    let mut diagnostics = Diagnostics::default();
    let mut applied_fixes = Vec::new();

    let workstreams = structure::collect_workstreams(store, options.workstream.as_deref())?;

    let _run_config = structure::read_run_config(store, &mut diagnostics)?;

    structure::structural_checks(
        structure::StructuralInputs {
            store,
            jules_path: jules_path.clone(),
            root: &root,
            workstreams: &workstreams,
            issue_labels: &issue_labels,
            event_states: &event_states,
            options: &options,
            applied_fixes: &mut applied_fixes,
        },
        &mut diagnostics,
    );

    let prompt_entries = schema::collect_prompt_entries(store, &jules_path, &mut diagnostics)?;

    schema::schema_checks(
        schema::SchemaInputs {
            store,
            jules_path: &jules_path,
            root: &root,
            workstreams: &workstreams,
            issue_labels: &issue_labels,
            event_states: &event_states,
            event_confidence: &event_confidence,
            issue_priorities: &issue_priorities,
            prompt_entries: &prompt_entries,
        },
        &mut diagnostics,
    );

    naming::naming_checks(store, &jules_path, &workstreams, &issue_labels, &event_states, &mut diagnostics);

    let semantic_context =
        semantic::semantic_context(store, &jules_path, &workstreams, &issue_labels, &mut diagnostics);
    semantic::semantic_checks(store, &jules_path, &workstreams, &semantic_context, &mut diagnostics);

    quality::quality_checks(
        store,
        &jules_path,
        &workstreams,
        &issue_labels,
        &event_states,
        &mut diagnostics,
    );

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

    if !applied_fixes.is_empty() {
        println!("\nApplied fixes:");
        for fix in &applied_fixes {
            println!("- {}", fix);
        }
    }

    Ok(DoctorOutcome { errors, warnings, exit_code })
}
