use crate::app::commands::run::RunOptions;
use crate::app::configuration::load_schedule;
use crate::domain::{AppError, Layer};
use crate::ports::{GitHubPort, GitPort, WorkspaceStore};
use std::path::Path;

use crate::app::commands::workflow::run::options::{RunResults, WorkflowRunOptions};

pub(super) fn execute<W, G, H, F>(
    store: &W,
    options: &WorkflowRunOptions,
    jules_path: &Path,
    git: &G,
    github: &H,
    run_layer: &mut F,
) -> Result<RunResults, AppError>
where
    W: WorkspaceStore + Clone + Send + Sync + 'static,
    G: GitPort,
    H: GitHubPort,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };
    let schedule = load_schedule(store)?;

    if !schedule.enabled {
        eprintln!("Schedule is disabled, skipping");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    let roles = schedule.innovators.as_ref().map(|l| l.enabled_roles()).unwrap_or_default();
    if roles.is_empty() {
        eprintln!("No enabled innovators roles");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    for role in roles {
        let run_options = RunOptions {
            layer: Layer::Innovators,
            role: Some(role.as_str().to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: options.mock,
            task: options.task.clone(),
        };

        eprintln!("Executing: innovators --role {}{}", role, mock_suffix);
        run_layer(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}
