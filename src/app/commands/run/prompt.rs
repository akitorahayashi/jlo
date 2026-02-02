//! Prompt assembly for Jules agents.

use std::fs;
use std::path::Path;

use crate::domain::{AppError, Layer};

/// Assemble the full prompt for a role in a multi-role layer.
pub fn assemble_prompt(jules_path: &Path, layer: Layer, role: &str) -> Result<String, AppError> {
    let role_dir = jules_path.join("roles").join(layer.dir_name()).join(role);
    let prompt_path = role_dir.join("prompt.yml");

    if !prompt_path.exists() {
        return Err(AppError::RoleNotFound(format!(
            "{}/{} (prompt.yml not found)",
            layer.dir_name(),
            role
        )));
    }

    let mut prompt_parts = Vec::new();

    // 1. Read prompt.yml
    let prompt_content = fs::read_to_string(&prompt_path)?;
    prompt_parts.push(prompt_content);

    // 2. Read contracts.yml if it exists
    let contracts_path = jules_path.join("roles").join(layer.dir_name()).join("contracts.yml");
    if contracts_path.exists() {
        let contracts = fs::read_to_string(&contracts_path)?;
        prompt_parts.push(format!("\n---\n# Layer Contracts\n{}", contracts));
    }

    // 3. Read role.yml if exists (observers)
    let role_yml_path = role_dir.join("role.yml");
    if role_yml_path.exists() {
        let role_config = fs::read_to_string(&role_yml_path)?;
        prompt_parts.push(format!("\n---\n# Role Configuration\n{}", role_config));
    }

    // 4. For observers, include changes/latest.yml if present (Narrator output)
    if layer == Layer::Observers {
        let changes_path = jules_path.join("changes").join("latest.yml");
        if changes_path.exists()
            && let Ok(changes_content) = fs::read_to_string(&changes_path)
        {
            prompt_parts.push(format!("\n---\n# Recent Codebase Changes\n{}", changes_content));
        }
    }

    Ok(prompt_parts.join("\n"))
}

/// Assemble the prompt for a single-role layer (Narrator, Planners, Implementers).
///
/// Single-role layers have prompt.yml directly in the layer directory,
/// not in a role subdirectory.
pub fn assemble_single_role_prompt(jules_path: &Path, layer: Layer) -> Result<String, AppError> {
    let layer_dir = jules_path.join("roles").join(layer.dir_name());
    let prompt_path = layer_dir.join("prompt.yml");

    if !prompt_path.exists() {
        return Err(AppError::RoleNotFound(format!("{}/prompt.yml not found", layer.dir_name())));
    }

    let mut prompt_parts = Vec::new();

    // 1. Read prompt.yml
    let prompt_content = fs::read_to_string(&prompt_path)?;
    prompt_parts.push(prompt_content);

    // 2. Read contracts.yml if it exists
    let contracts_path = layer_dir.join("contracts.yml");
    if contracts_path.exists() {
        let contracts = fs::read_to_string(&contracts_path)?;
        prompt_parts.push(format!("\n---\n# Layer Contracts\n{}", contracts));
    }

    Ok(prompt_parts.join("\n"))
}
