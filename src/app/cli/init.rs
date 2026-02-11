//! Init command implementation.

use crate::domain::AppError;

pub fn run_init(remote: bool, self_hosted: bool) -> Result<(), AppError> {
    let mode = if remote {
        crate::domain::WorkflowRunnerMode::remote()
    } else if self_hosted {
        crate::domain::WorkflowRunnerMode::self_hosted()
    } else {
        return Err(AppError::MissingArgument(
            "Runner mode is required. Use --remote or --self-hosted.".into(),
        ));
    };
    let label = mode.label().to_string();
    crate::app::api::init(mode)?;
    println!("✅ Initialized .jlo/ control plane and workflow scaffold ({})", label);
    Ok(())
}
