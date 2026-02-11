//! Implementer layer prompt assembly.
//!
//! Builds the PromptContext for implementer prompts. The `task` variable is
//! resolved from the label-specific task file (e.g. tasks/bugs.yml, tasks/feats.yml)
//! before calling this assembler.

use std::path::Path;

use crate::domain::{AppError, Layer, PromptAssetLoader};

use super::engine::{AssembledPrompt, PromptContext, assemble_prompt};

/// Implementer-specific context for prompt assembly.
pub struct ImplementerPromptInput<'a> {
    /// Pre-resolved label-specific task file content.
    pub task: &'a str,
}

/// Assemble the implementer prompt with label-resolved task context.
pub fn assemble<L>(
    jules_path: &Path,
    input: &ImplementerPromptInput<'_>,
    loader: &L,
) -> Result<AssembledPrompt, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    let context = PromptContext::new().with_var("task", input.task);

    assemble_prompt(jules_path, Layer::Implementer, &context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))
}
