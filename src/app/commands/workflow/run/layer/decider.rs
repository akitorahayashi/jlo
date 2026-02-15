use crate::app::commands::run::RunOptions;
use crate::domain::PromptAssetLoader;
use crate::domain::workspace::paths::jules;
use crate::domain::{AppError, Layer};
use crate::ports::{GitHubPort, GitPort, JloStorePort, JulesStorePort, RepositoryFilesystemPort};
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
    W: RepositoryFilesystemPort
        + JloStorePort
        + JulesStorePort
        + PromptAssetLoader
        + Clone
        + Send
        + Sync
        + 'static,
    G: GitPort,
    H: GitHubPort,
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    if !options.mock && !has_pending_events(jules_path)? {
        eprintln!("No pending events, skipping decider");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    let run_options = RunOptions {
        layer: Layer::Decider,
        role: None,
        prompt_preview: false,
        branch: None,
        requirement: None,
        mock: options.mock,
        task: options.task.clone(),
    };

    eprintln!("Executing: decider{}", if options.mock { " (mock)" } else { "" });
    run_layer(jules_path, run_options, git, github, store)?;

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}

fn has_pending_events(jules_path: &Path) -> Result<bool, AppError> {
    let pending_dir = jules::exchange_dir(jules_path).join("events/pending");
    if !pending_dir.exists() {
        return Ok(false);
    }
    let entries = std::fs::read_dir(&pending_dir)?;
    for entry in entries {
        let entry = entry?;
        if entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "yml") {
            return Ok(true);
        }
    }
    Ok(false)
}
