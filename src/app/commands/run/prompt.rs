//! Prompt assembly for Jules agents.
//!
//! Single-role layers (narrator, decider, planner, implementer) use the
//! generic functions below. Multi-role layers (observers, innovators) use
//! their dedicated domain assemblers in `domain::prompt_assembly`.

use std::path::Path;

use crate::domain::{
    AppError, Layer, PromptAssetLoader, PromptContext, assemble_prompt as assemble_prompt_domain,
    assemble_with_issue,
};

/// Assemble the prompt for a single-role layer (Narrator, Planner, Implementer, Decider).
///
/// Single-role layers use the prompt template directly in the layer directory
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
/// This is used for planner and implementer where the issue content is
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
