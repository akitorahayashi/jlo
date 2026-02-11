//! Innovator layer prompt assembly.
//!
//! Builds the PromptContext for innovator prompts: role, phase, and task content
//! resolved from the phase-specific task file (create_idea.yml or refine_proposal.yml).

use std::path::Path;

use crate::domain::{AppError, Layer, PromptAssetLoader};

use super::engine::{AssembledPrompt, PromptContext, assemble_prompt};

/// Innovator-specific context for prompt assembly.
pub struct InnovatorPromptInput<'a> {
    /// The innovator role id (e.g. "ux", "arch").
    pub role: &'a str,
    /// Execution phase: "creation" or "refinement".
    pub phase: &'a str,
    /// Pre-resolved task file content for this phase.
    pub task: &'a str,
}

/// Assemble the innovator prompt with layer-specific context.
pub fn assemble<L>(
    jules_path: &Path,
    input: &InnovatorPromptInput<'_>,
    loader: &L,
) -> Result<AssembledPrompt, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    let context = PromptContext::new()
        .with_var("role", input.role)
        .with_var("phase", input.phase)
        .with_var("task", input.task);

    assemble_prompt(jules_path, Layer::Innovators, &context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))
}
