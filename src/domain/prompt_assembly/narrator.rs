//! Narrator layer prompt assembly.
//!
//! Builds the PromptContext for narrator prompts: run_mode, range_description,
//! and commits_since_cursor (non-empty only for overwrite mode).

use crate::domain::{AppError, Layer, PromptAssetLoader};

use super::engine::{AssembledPrompt, PromptContext, assemble_prompt};

/// Narrator-specific context for prompt assembly.
#[allow(dead_code)]
pub struct NarratorPromptInput {
    /// "bootstrap" or "overwrite".
    pub run_mode: String,
    /// Human-readable description of the commit range.
    pub range_description: String,
    /// Commit list (sha + message) since cursor. Empty for bootstrap.
    pub commits_since_cursor: String,
}

/// Assemble the narrator prompt with layer-specific context.
#[allow(dead_code)]
pub fn assemble<L>(
    jules_path: &std::path::Path,
    input: &NarratorPromptInput,
    loader: &L,
) -> Result<AssembledPrompt, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    let context = PromptContext::new()
        .with_var("run_mode", &input.run_mode)
        .with_var("range_description", &input.range_description)
        .with_var("commits_since_cursor", &input.commits_since_cursor);

    assemble_prompt(jules_path, Layer::Narrator, &context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))
}
