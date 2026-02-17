use crate::app::commands::run::{RunOptions, RunRuntimeOptions};
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
    F: FnMut(&Path, RunOptions, RunRuntimeOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };
    let schedule = load_schedule(store)?;

    let roles = schedule.innovators.as_ref().map(|l| l.enabled_roles()).unwrap_or_default();
    if roles.is_empty() {
        eprintln!("No enabled innovators roles");
        return Ok(RunResults::skipped("No enabled innovators roles"));
    }

    let mut success_count: u32 = 0;
    for role in roles {
        let run_options = RunOptions {
            layer: Layer::Innovators,
            role: Some(role.as_str().to_string()),
            requirement: None,
            task: options.task.clone(),
        };
        let runtime = RunRuntimeOptions {
            prompt_preview: false,
            branch: options.branch.clone(),
            mock: options.mock,
            no_cleanup: false,
        };

        eprintln!("Executing: innovators --role {}{}", role, mock_suffix);
        run_layer(jules_path, run_options, runtime, git, github, store)?;
        success_count += 1;
    }

    Ok(RunResults::with_count(success_count))
}
