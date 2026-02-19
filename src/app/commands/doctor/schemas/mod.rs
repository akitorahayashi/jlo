pub mod changes;
pub mod contracts;
mod dates;
pub mod events;
pub mod placeholders;
pub mod proposals;
pub mod requirements;
pub mod roles;

use std::path::{Path, PathBuf};

use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::structure::list_subdirs;
use crate::app::commands::doctor::yaml::read_yaml_files;
use crate::domain::{AppError, Layer};

use self::changes::validate_changes_file;
use self::contracts::validate_contracts;
use self::events::validate_event_file;
use self::placeholders::check_placeholders_file;
use self::proposals::validate_innovator_proposal;
use self::requirements::validate_requirement_file;
use self::roles::{validate_innovator_role_file, validate_role_file};

#[derive(Debug, Clone)]
pub(crate) struct PromptEntry {
    pub path: PathBuf,
    pub contracts: Vec<String>,
}

pub struct SchemaInputs<'a> {
    pub jules_path: &'a Path,
    pub root: &'a Path,
    pub issue_labels: &'a [String],
    pub event_states: &'a [String],
    pub event_confidence: &'a [String],
    pub issue_priorities: &'a [String],
    pub prompt_entries: &'a [PromptEntry],
}

pub fn collect_prompt_entries(
    _jules_path: &Path,
    _diagnostics: &mut Diagnostics,
) -> Result<Vec<PromptEntry>, AppError> {
    // Prompt entries (templates, contracts, tasks) are now embedded in the binary
    // via src/assets/prompt-assemble/. No filesystem-based collection needed.
    Ok(Vec::new())
}

pub fn schema_checks(inputs: SchemaInputs<'_>, diagnostics: &mut Diagnostics) {
    for entry in inputs.prompt_entries {
        for contract in &entry.contracts {
            let contract_path = inputs.root.join(contract);
            if !contract_path.exists() {
                diagnostics.push_error(
                    entry.path.display().to_string(),
                    format!("Contract not found: {}", contract),
                );
            }
        }
    }

    let changes_path = crate::domain::exchange::paths::exchange_changes(inputs.jules_path);
    if changes_path.exists() {
        validate_changes_file(&changes_path, diagnostics);
    }

    // Validate embedded contracts for each layer
    for layer in Layer::ALL {
        let catalog_path = format!("{}/contracts.yml", layer.dir_name());
        if let Some(content) =
            crate::adapters::catalogs::prompt_assemble_assets::read_prompt_assemble_asset(
                &catalog_path,
            )
            && let Ok(data) = serde_yaml::from_str::<serde_yaml::Value>(&content)
            && let Some(mapping) = data.as_mapping()
        {
            let label = format!("prompt-assemble://{}", catalog_path);
            validate_contracts(mapping, Path::new(&label), layer, diagnostics);
        }

        // Validate role definitions in .jlo/roles/ for multi-role layers
        if layer == Layer::Observers {
            let jlo_layer_dir = crate::domain::roles::paths::layer_dir(inputs.root, layer);
            if jlo_layer_dir.exists() {
                for role_dir in list_subdirs(&jlo_layer_dir, diagnostics) {
                    let role_path = role_dir.join("role.yml");
                    if role_path.exists() {
                        validate_role_file(&role_path, &role_dir, diagnostics);
                    }
                }
            }
        }

        if layer == Layer::Innovators {
            let jlo_layer_dir = crate::domain::roles::paths::layer_dir(inputs.root, layer);
            if jlo_layer_dir.exists() {
                for role_dir in list_subdirs(&jlo_layer_dir, diagnostics) {
                    let role_path = role_dir.join("role.yml");
                    if role_path.exists() {
                        validate_innovator_role_file(&role_path, &role_dir, diagnostics);
                    }
                }
            }
        }
    }

    // Validate flat exchange directory
    {
        for state in inputs.event_states {
            let state_dir =
                crate::domain::exchange::events::paths::events_state_dir(inputs.jules_path, state);
            for entry in read_yaml_files(&state_dir, diagnostics) {
                validate_event_file(&entry, state, inputs.event_confidence, diagnostics);
                check_placeholders_file(&entry, diagnostics);
            }
        }

        {
            let requirements_dir =
                crate::domain::exchange::requirements::paths::requirements_dir(inputs.jules_path);
            for entry in read_yaml_files(&requirements_dir, diagnostics) {
                validate_requirement_file(
                    &entry,
                    inputs.issue_labels,
                    inputs.issue_priorities,
                    diagnostics,
                );
                check_placeholders_file(&entry, diagnostics);
            }
        }

        let proposals_dir =
            crate::domain::exchange::proposals::paths::proposals_dir(inputs.jules_path);
        for proposal_path in read_yaml_files(&proposals_dir, diagnostics) {
            validate_innovator_proposal(&proposal_path, diagnostics);
            check_placeholders_file(&proposal_path, diagnostics);
        }
    }
}
