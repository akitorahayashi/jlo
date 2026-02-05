//! Prompt assembly for Jules agents.
//!
//! This module provides a unified interface for prompt assembly, delegating
//! to the prompt_assembly service for asset-driven prompt composition.

use crate::domain::{AppError, Layer, PromptContext};
use crate::ports::WorkspaceStore;
use crate::services::application::prompt_assembly;

/// Assemble the full prompt for a role in a multi-role layer.
///
/// Multi-role layers (observers, deciders) require workstream and role context.
pub fn assemble_prompt(
    workspace: &impl WorkspaceStore,
    layer: Layer,
    role: &str,
    workstream: &str,
) -> Result<String, AppError> {
    let context = PromptContext::new().with_var("workstream", workstream).with_var("role", role);
    Ok(prompt_assembly::assemble_prompt(workspace, layer, &context)?.content)
}

/// Assemble the prompt for a single-role layer (Narrator, Planners, Implementers).
///
/// Single-role layers have prompt.yml directly in the layer directory,
/// not in a role subdirectory. They do not require workstream/role context.
pub fn assemble_single_role_prompt(
    workspace: &impl WorkspaceStore,
    layer: Layer,
) -> Result<String, AppError> {
    Ok(prompt_assembly::assemble_prompt(workspace, layer, &PromptContext::new())?.content)
}

/// Assemble the prompt for an issue-driven layer with embedded issue content.
///
/// This is used for planners and implementers where the issue content is
/// appended to the base prompt.
#[allow(dead_code)]
pub fn assemble_issue_prompt(
    workspace: &impl WorkspaceStore,
    layer: Layer,
    issue_content: &str,
) -> Result<String, AppError> {
    Ok(prompt_assembly::assemble_with_issue(workspace, layer, issue_content)?.content)
}
