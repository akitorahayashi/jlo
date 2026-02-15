use crate::app::commands::run::RunOptions;
use crate::app::commands::workflow::run::input::load_schedule;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer};
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};
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
    W: RepositoryFilesystem
        + JloStore
        + JulesStore
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
    G: Git,
    H: GitHub,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };
    let schedule = load_schedule(store)?;

    let roles = schedule.observers.enabled_roles();
    if roles.is_empty() {
        eprintln!("No enabled observers roles");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    for role in roles {
        let run_options = RunOptions {
            layer: Layer::Observers,
            role: Some(role.as_str().to_string()),
            prompt_preview: false,
            branch: None,
            requirement: None,
            mock: options.mock,
            task: options.task.clone(),
        };

        eprintln!("Executing: observers --role {}{}", role, mock_suffix);
        run_layer(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}
