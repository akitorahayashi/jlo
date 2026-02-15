use crate::app::commands::run::RunOptions;
use crate::app::commands::workflow::run::issue_routing::find_requirements;
use crate::app::commands::workflow::run::options::{RunResults, WorkflowRunOptions};
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer};
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};
use std::path::Path;

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
    let requirements = find_requirements(store, Layer::Planner)?;

    if requirements.is_empty() {
        eprintln!("No requirements found for planner");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    for requirement_path in requirements {
        let run_options = RunOptions {
            layer: Layer::Planner,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: Some(requirement_path.clone()),
            mock: options.mock,
            task: options.task.clone(),
        };

        eprintln!("Executing: planner {}{}", requirement_path.display(), mock_suffix);
        run_layer(jules_path, run_options, git, github, store)?;
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}
