//! Prompt assembly for Jules agents.
//!
//! This module provides a unified interface for prompt assembly, delegating
//! to the prompt_assembly service for asset-driven prompt composition.

use std::path::Path;

use crate::domain::{AppError, Layer, PromptContext};
use crate::services::application::prompt_assembly::{self, RealPromptFs};

/// Assemble the full prompt for a role in a multi-role layer.
///
/// Multi-role layers (observers, deciders) require workstream and role context.
pub fn assemble_prompt(
    jules_path: &Path,
    layer: Layer,
    role: &str,
    workstream: &str,
) -> Result<String, AppError> {
    let context = PromptContext::new().with_var("workstream", workstream).with_var("role", role);
    let fs = RealPromptFs;

    Ok(prompt_assembly::assemble_prompt(jules_path, layer, &context, &fs)?.content)
}

/// Assemble the prompt for a single-role layer (Narrator, Planners, Implementers).
///
/// Single-role layers have prompt.yml directly in the layer directory,
/// not in a role subdirectory. They do not require workstream/role context.
pub fn assemble_single_role_prompt(jules_path: &Path, layer: Layer) -> Result<String, AppError> {
    let fs = RealPromptFs;
    Ok(prompt_assembly::assemble_prompt(jules_path, layer, &PromptContext::new(), &fs)?.content)
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
) -> Result<String, AppError> {
    let fs = RealPromptFs;
    Ok(prompt_assembly::assemble_with_issue(jules_path, layer, issue_content, &fs)?.content)
}
