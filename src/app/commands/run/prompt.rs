//! Prompt assembly for Jules agents.
//!
//! This module provides a unified interface for prompt assembly, delegating
//! to the prompt_assembly domain logic.

use std::path::Path;

use crate::domain::{
    assemble_prompt as assemble_prompt_domain, assemble_with_issue, AppError, Layer, PromptContext,
    PromptAssetLoader,
};

/// Assemble the full prompt for a role in a multi-role layer.
///
/// Multi-role layers (observers, deciders) require workstream and role context.
pub fn assemble_prompt(
    jules_path: &Path,
    layer: Layer,
    role: &str,
    workstream: &str,
    loader: &impl PromptAssetLoader,
) -> Result<String, AppError> {
    let context = PromptContext::new().with_var("workstream", workstream).with_var("role", role);

    Ok(assemble_prompt_domain(jules_path, layer, &context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .content)
}

/// Assemble the prompt for a single-role layer (Narrator, Planners, Implementers).
///
/// Single-role layers have prompt.yml directly in the layer directory,
/// not in a role subdirectory. They do not require workstream/role context.
pub fn assemble_single_role_prompt(
    jules_path: &Path,
    layer: Layer,
    loader: &impl PromptAssetLoader,
) -> Result<String, AppError> {
    Ok(assemble_prompt_domain(jules_path, layer, &PromptContext::new(), loader)
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .content)
}

/// Assemble the prompt for an issue-driven layer with embedded issue content.
///
/// This is used for planners and implementers where the issue content is
/// appended to the base prompt.
#[allow(dead_code)]
pub fn assemble_issue_prompt(
    jules_path: &Path,
    layer: Layer,
    issue_content: &str,
    loader: &impl PromptAssetLoader,
) -> Result<String, AppError> {
    Ok(assemble_with_issue(jules_path, layer, issue_content, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .content)
}
