pub mod decider;
pub mod implementer;
pub mod innovators;
pub mod integrator;
pub mod narrator;
pub mod observers;
pub mod planner;

use crate::domain::AppError;
use crate::domain::prompt_assemble::{PromptAssemblyError, PromptAssetLoader, SeedOp};

/// Execute deferred seed operations collected during prompt assembly.
///
/// For each [`SeedOp`], ensures the destination directory exists and copies the
/// schema file to the target path. Required ops propagate errors; optional ops
/// ignore failures silently.
pub(super) fn execute_seed_ops<L: PromptAssetLoader>(
    ops: Vec<SeedOp>,
    loader: &L,
) -> Result<(), AppError> {
    for op in ops {
        let make_error = |err: std::io::Error| {
            AppError::PromptAssembly(PromptAssemblyError::SchemaSeedError {
                path: op.to.to_string_lossy().to_string(),
                reason: err.to_string(),
            })
        };

        if let Some(parent) = op.to.parent()
            && let Err(err) = loader.ensure_asset_dir(parent)
        {
            if op.required {
                return Err(make_error(err));
            }
            continue;
        }

        if let Err(err) = loader.copy_asset(&op.from, &op.to)
            && op.required
        {
            return Err(make_error(err));
        }
    }
    Ok(())
}
