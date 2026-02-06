//! Doctor command implementation.

use crate::domain::AppError;

pub fn run_doctor(fix: bool, strict: bool, workstream: Option<String>) -> Result<i32, AppError> {
    let options = crate::DoctorOptions { fix, strict, workstream };
    let outcome = crate::api::doctor(options)?;

    Ok(outcome.exit_code)
}
