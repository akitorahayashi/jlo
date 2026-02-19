//! Init command implementation.

use crate::domain::AppError;

pub fn run_init(mode: super::InitMode) -> Result<(), AppError> {
    let mode = match mode {
        super::InitMode::Remote => crate::domain::WorkflowRunnerMode::remote(),
        super::InitMode::SelfHosted => crate::domain::WorkflowRunnerMode::self_hosted(),
    };
    crate::app::api::init(&mode)?;
    println!("✅ Initialized .jlo/ control plane and workflow scaffold ({})", mode.label());
    Ok(())
}
