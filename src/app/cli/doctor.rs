//! Doctor command implementation.

use crate::domain::AppError;

pub fn run_doctor(strict: bool) -> Result<i32, AppError> {
    let options = crate::DoctorOptions { strict };
    let outcome = crate::app::api::doctor(options)?;

    Ok(outcome.exit_code)
}
