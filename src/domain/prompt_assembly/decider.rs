//! Decider layer prompt assembly.
//!
//! The decider template uses only `include_required` — no runtime variables.
//! This assembler provides a typed entry point consistent with the other layers.

use std::path::Path;

use crate::domain::{AppError, Layer, PromptAssetLoader};

use super::engine::{AssembledPrompt, PromptContext, assemble_prompt};

/// Assemble the decider prompt.
///
/// No additional context variables are required — the template resolves
/// everything from static includes.
#[allow(dead_code)]
pub fn assemble<L>(jules_path: &Path, loader: &L) -> Result<AssembledPrompt, AppError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
{
    assemble_prompt(jules_path, Layer::Deciders, &PromptContext::new(), loader)
        .map_err(|e| AppError::InternalError(e.to_string()))
}
