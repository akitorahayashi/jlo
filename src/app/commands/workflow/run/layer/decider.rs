use crate::app::commands::run::{RunOptions, RunRuntimeOptions};
use crate::domain::PromptAssetLoader;
use crate::domain::layers::execute::policy::has_pending_events;
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
    if !options.mock && !has_pending_events(jules_path)? {
        eprintln!("No pending events, skipping decider");
        return Ok(RunResults::skipped("No pending events"));
    }

    let run_options = RunOptions {
        layer: Layer::Decider,
        role: None,
        requirement: None,
        task: options.task.clone(),
    };
    let runtime = RunRuntimeOptions {
        prompt_preview: false,
        branch: None,
        mock: options.mock,
        no_cleanup: false,
    };

    eprintln!("Executing: decider{}", if options.mock { " (mock)" } else { "" });
    run_layer(jules_path, run_options, runtime, git, github, store)?;

    Ok(RunResults::with_count(1))
}
