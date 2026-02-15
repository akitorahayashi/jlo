use crate::app::commands::run::RunOptions;
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
    let run_options = RunOptions {
        layer: Layer::Narrator,
        role: None,
        prompt_preview: false,
        branch: None,
        requirement: None,
        mock: options.mock,
        task: options.task.clone(),
    };

    eprintln!("Executing: narrator{}", if options.mock { " (mock)" } else { "" });
    run_layer(jules_path, run_options, git, github, store)?;

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}
