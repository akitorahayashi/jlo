use crate::domain::{AppError, WorkstreamSchedule, JULES_DIR};
use crate::ports::WorkspaceStore;

pub fn load_schedule(workspace: &impl WorkspaceStore, workstream: &str) -> Result<WorkstreamSchedule, AppError> {
    let path = format!("{}/workstreams/{}/scheduled.toml", JULES_DIR, workstream);

    let content = workspace.read_file(&path).map_err(|err| {
        // Map generic Io/NotFound to ScheduleConfigMissing if needed,
        // but AppError::Io usually wraps std::io::Error.
        // WorkspaceStore::read_file returns AppError.
        // If we want to preserve specific error behavior:
        match err {
             AppError::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
                 AppError::ScheduleConfigMissing(path.clone())
             }
             _ => err,
        }
    })?;
    Ok(WorkstreamSchedule::parse_toml(&content)?)
}
