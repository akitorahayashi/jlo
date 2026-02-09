//! Init command implementation.

use crate::domain::AppError;

pub fn run_init(remote: bool, self_hosted: bool) -> Result<(), AppError> {
    let mode = if remote {
        crate::domain::WorkflowRunnerMode::Remote
    } else if self_hosted {
        crate::domain::WorkflowRunnerMode::SelfHosted
    } else {
        return Err(AppError::MissingArgument(
            "Runner mode is required. Use --remote or --self-hosted.".into(),
        ));
    };
    crate::app::api::init(mode)?;
    println!("✅ Initialized .jlo/ control plane and workflow scaffold ({})", mode.label());
    Ok(())
}
