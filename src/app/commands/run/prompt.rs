//! Prompt assembly for Jules agents.
//!
//! This module provides a unified interface for prompt assembly, delegating
//! to the prompt_assembly domain logic.

use std::path::Path;

use crate::domain::identifiers::validation::validate_safe_path_component;
use crate::domain::{
    AppError, Layer, PromptAssetLoader, PromptContext, assemble_prompt as assemble_prompt_domain,
    assemble_with_issue,
};

/// Assemble the full prompt for a role in a multi-role layer.
///
/// Multi-role layers (observers, deciders, innovators) require role context.
/// Innovators additionally require a phase context variable.
pub fn assemble_prompt<L>(
    jules_path: &Path,
    layer: Layer,
    role: &str,
    phase: Option<&str>,
    loader: &L,
) -> Result<String, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    // Validate role to prevent prompt injection and path traversal
    if !validate_safe_path_component(role) {
        return Err(AppError::Validation(format!(
            "Invalid role '{}': must be alphanumeric with hyphens or underscores",
            role
        )));
    }

    let mut context = PromptContext::new().with_var("role", role);
    if layer == Layer::Innovators {
        let phase_val = phase.ok_or_else(|| {
            AppError::MissingArgument(
                "--phase is required for innovators (creation or refinement)".to_string(),
            )
        })?;
        context = context.with_var("phase", phase_val);
    }
    Ok(assemble_prompt_domain(jules_path, layer, &context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .content)
}

/// Assemble the prompt for a single-role layer (Narrator, Planners, Implementers).
///
/// Single-role layers use prompt_assembly.j2 directly in the layer directory
/// and do not require role context.
pub fn assemble_single_role_prompt<L>(
    jules_path: &Path,
    layer: Layer,
    loader: &L,
) -> Result<String, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    assemble_single_role_prompt_with_context(jules_path, layer, &PromptContext::new(), loader)
}

/// Assemble the prompt for a single-role layer using an explicit context.
pub fn assemble_single_role_prompt_with_context<L>(
    jules_path: &Path,
    layer: Layer,
    context: &PromptContext,
    loader: &L,
) -> Result<String, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    Ok(assemble_prompt_domain(jules_path, layer, context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .content)
}

/// Assemble the prompt for an issue-driven layer with embedded issue content.
///
/// This is used for planners and implementers where the issue content is
/// appended to the base prompt.
#[allow(dead_code)]
pub fn assemble_issue_prompt<L>(
    jules_path: &Path,
    layer: Layer,
    issue_content: &str,
    loader: &L,
) -> Result<String, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    Ok(assemble_with_issue(jules_path, layer, issue_content, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))?
        .content)
}
