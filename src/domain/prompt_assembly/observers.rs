//! Observer layer prompt assembly.
//!
//! Builds the PromptContext for observer prompts. The bridge_task variable
//! is populated only when innovator ideas exist, controlled by the caller.

use crate::domain::{AppError, Layer, PromptAssetLoader};

use super::engine::{AssembledPrompt, PromptContext, assemble_prompt};

/// Observer-specific context for prompt assembly.
#[allow(dead_code)]
pub struct ObserverPromptInput {
    /// The observer role id (e.g. "taxonomy", "qa").
    pub role: String,
    /// Content of tasks/bridge_comments.yml, or empty if no innovator ideas exist.
    pub bridge_task: String,
}

/// Assemble the observer prompt with layer-specific context.
#[allow(dead_code)]
pub fn assemble<L>(
    jules_path: &std::path::Path,
    input: &ObserverPromptInput,
    loader: &L,
) -> Result<AssembledPrompt, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    let context = PromptContext::new()
        .with_var("role", &input.role)
        .with_var("bridge_task", &input.bridge_task);

    assemble_prompt(jules_path, Layer::Observers, &context, loader)
        .map_err(|e| AppError::InternalError(e.to_string()))
}
