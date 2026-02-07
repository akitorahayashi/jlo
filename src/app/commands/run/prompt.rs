//! Prompt assembly for Jules agents.
//!
//! This module provides a unified interface for prompt assembly, delegating
//! to the prompt_assembly domain logic.

use std::path::Path;

use crate::adapters::template::MinijinjaTemplateRenderer;
use crate::domain::identities::validation::{validate_identifier, validate_safe_path_component};
use crate::domain::{
    AppError, Layer, PromptAssetLoader, PromptContext, assemble_prompt as assemble_prompt_domain,
    assemble_with_issue,
};

/// Assemble the full prompt for a role in a multi-role layer.
///
/// Multi-role layers (observers, deciders, innovators) require workstream and role context.
/// Innovators additionally require a phase context variable.
pub fn assemble_prompt(
    jules_path: &Path,
    layer: Layer,
    role: &str,
    workstream: &str,
    phase: Option<&str>,
    loader: &impl PromptAssetLoader,
) -> Result<String, AppError> {
    // Validate role and workstream to prevent prompt injection
    if !validate_identifier(role, false) {
        return Err(AppError::Validation(format!(
            "Invalid role '{}': must be alphanumeric with hyphens or underscores",
            role
        )));
    }
    if !validate_safe_path_component(workstream) {
        return Err(AppError::Validation(format!(
            "Invalid workstream '{}': must be alphanumeric with hyphens or underscores",
            workstream
        )));
    }

    let mut context =
        PromptContext::new().with_var("workstream", workstream).with_var("role", role);
    if let Some(phase_val) = phase {
        context = context.with_var("phase", phase_val);
    }
    let renderer = MinijinjaTemplateRenderer::new();

    Ok(assemble_prompt_domain(jules_path, layer, &context, loader, &renderer)
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
    let renderer = MinijinjaTemplateRenderer::new();
    Ok(assemble_prompt_domain(jules_path, layer, &PromptContext::new(), loader, &renderer)
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
    let renderer = MinijinjaTemplateRenderer::new();
    Ok(assemble_with_issue(jules_path, layer, issue_content, loader, &renderer)
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .content)
}
