use crate::app::commands::run::RunOptions;
use crate::app::commands::workflow::gh::push::{
    PushWorkerBranchOptions, execute as push_worker_branch,
};
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
    F: FnMut(&Path, RunOptions, &G, &H, &W) -> Result<(), AppError>,
{
    let mock_suffix = if options.mock { " (mock)" } else { "" };
    let requirements = find_requirements(store, Layer::Implementer)?;

    if requirements.is_empty() {
        eprintln!("No requirements found for implementer");
        return Ok(RunResults { mock_pr_numbers: None, mock_branches: None });
    }

    for requirement_path in requirements {
        let run_options = RunOptions {
            layer: Layer::Implementer,
            role: None,
            prompt_preview: false,
            branch: None,
            requirement: Some(requirement_path.clone()),
            mock: options.mock,
            task: options.task.clone(),
        };

        eprintln!("Executing: implementer {}{}", requirement_path.display(), mock_suffix);
        run_layer(jules_path, run_options, git, github, store)?;
    }

    let defer_worker_merge = std::env::var("JLO_DEFER_WORKER_MERGE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if defer_worker_merge && !options.mock {
        let out = push_worker_branch(PushWorkerBranchOptions {
            change_token: "implementer-cleanup-batch".to_string(),
            commit_message: "jules: clean implementer requirements".to_string(),
            pr_title: "chore: clean implementer requirements".to_string(),
            pr_body: "Automated cleanup for processed implementer requirements and source events."
                .to_string(),
        })?;

        if out.applied {
            let pr_number = out.pr_number.ok_or_else(|| {
                AppError::InternalError(
                    "worker cleanup push reported applied=true without pr_number".to_string(),
                )
            })?;
            eprintln!("Merged consolidated implementer cleanup PR #{}", pr_number);
        } else if let Some(reason) = out.skipped_reason {
            eprintln!("Skipped consolidated implementer cleanup merge: {}", reason);
        }
    }

    Ok(RunResults { mock_pr_numbers: None, mock_branches: None })
}
