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

    // 4. Read notes if directory exists
    let notes_path = role_dir.join("notes");
    if notes_path.exists() && notes_path.is_dir() {
        let mut note_contents = Vec::new();
        for entry in fs::read_dir(&notes_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Ok(content) = fs::read_to_string(&path)
            {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                note_contents.push(format!("## {}\n{}", filename, content));
            }
        }
        if !note_contents.is_empty() {
            prompt_parts.push(format!("\n---\n# Notes\n{}", note_contents.join("\n\n")));
        }
    }

    Ok(prompt_parts.join("\n"))
}

/// Assemble the prompt for a single-role layer (Planners, Implementers).
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
