//! Observer layer prompt assembly.
//!
//! Builds the PromptContext for observer prompts. The bridge_task variable
//! is populated only when innovator ideas exist, controlled by the caller.

use crate::domain::{AppError, Layer, PromptAssetLoader};

use super::engine::{AssembledPrompt, PromptContext, assemble_prompt};

/// Observer-specific context for prompt assembly.
pub struct ObserverPromptInput<'a> {
    /// The observer role id (e.g. "taxonomy", "qa").
    pub role: &'a str,
    /// Content of tasks/bridge_comments.yml, or empty if no innovator ideas exist.
    pub bridge_task: &'a str,
}

/// Assemble the observer prompt with layer-specific context.
pub fn assemble<L>(
    jules_path: &std::path::Path,
    input: &ObserverPromptInput<'_>,
    loader: &L,
) -> Result<AssembledPrompt, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    let context = PromptContext::new()
        .with_var("role", input.role)
        .with_var("bridge_task", input.bridge_task);

    assemble_prompt(jules_path, Layer::Observers, &context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))
}
