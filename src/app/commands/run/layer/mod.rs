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
        if let Some(parent) = op.to.parent() {
            match loader.ensure_asset_dir(parent) {
                Ok(()) => {}
                Err(err) if op.required => {
                    return Err(AppError::PromptAssembly(PromptAssemblyError::SchemaSeedError {
                        path: op.to.to_string_lossy().to_string(),
                        reason: err.to_string(),
                    }));
                }
                Err(_) => continue,
            }
        }
        match loader.copy_asset(&op.from, &op.to) {
            Ok(_) => {}
            Err(err) if op.required => {
                return Err(AppError::PromptAssembly(PromptAssemblyError::SchemaSeedError {
                    path: op.to.to_string_lossy().to_string(),
                    reason: err.to_string(),
                }));
            }
            Err(_) => {}
        }
    }
    Ok(())
}
