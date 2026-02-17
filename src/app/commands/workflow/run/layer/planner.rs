use crate::app::commands::run::{RunOptions, RunRuntimeOptions};
use crate::app::commands::workflow::run::options::{RunResults, WorkflowRunOptions};
use crate::app::commands::workflow::run::requirements_routing::find_requirements;
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
    F: FnMut(&Path, RunOptions, RunRuntimeOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };
    let requirements = find_requirements(store, Layer::Planner)?;

    if requirements.is_empty() {
        eprintln!("No requirements found for planner");
        return Ok(RunResults::skipped("No requirements found for planner"));
    }

    let mut success_count: u32 = 0;
    for requirement_path in requirements {
        let run_options = RunOptions {
            layer: Layer::Planner,
            role: None,
            requirement: Some(requirement_path.clone()),
            task: options.task.clone(),
        };
        let runtime = RunRuntimeOptions {
            prompt_preview: false,
            branch: options.branch.clone(),
            mock: options.mock,
            no_cleanup: false,
        };

        eprintln!("Executing: planner {}{}", requirement_path.display(), mock_suffix);
        run_layer(jules_path, run_options, runtime, git, github, store)?;
        success_count += 1;
    }

    Ok(RunResults::with_count(success_count))
}
