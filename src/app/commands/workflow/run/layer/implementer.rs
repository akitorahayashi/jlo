use crate::app::commands::run::{RunOptions, RunRuntimeOptions};
use crate::app::commands::workflow::exchange::{
    ExchangeCleanRequirementOptions, clean_requirement_apply_with_adapters,
};
use crate::app::commands::workflow::push::{
    PushWorkerBranchOptions, execute as push_worker_branch,
};
use crate::app::commands::workflow::run::options::{RunResults, WorkflowRunOptions};
use crate::app::commands::workflow::run::requirements_routing::find_requirements;
use crate::domain::PromptAssetLoader;
use crate::domain::{AppError, Layer};
use crate::ports::{Git, GitHub, JloStore, JulesStore, RepositoryFilesystem};
use std::path::{Path, PathBuf};

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
    let requirements = find_requirements(store, Layer::Implementer)?;

    if requirements.is_empty() {
        eprintln!("No requirements found for implementer");
        return Ok(RunResults::skipped("No requirements found for implementer"));
    }

    // Execute each requirement with no_cleanup=true, track successes
    let mut succeeded: Vec<PathBuf> = Vec::new();
    let mut first_error: Option<AppError> = None;

    for requirement_path in &requirements {
        let run_options = RunOptions {
            layer: Layer::Implementer,
            role: None,
            requirement: Some(requirement_path.clone()),
            task: options.task.clone(),
        };
        let runtime = RunRuntimeOptions {
            prompt_preview: false,
            branch: options.branch.clone(),
            mock: options.mock,
            no_cleanup: true,
        };

        eprintln!("Executing: implementer {}{}", requirement_path.display(), mock_suffix);
        match run_layer(jules_path, run_options, runtime, git, github, store) {
            Ok(()) => {
                succeeded.push(requirement_path.clone());
            }
            Err(e) => {
                eprintln!("Failed: implementer {} — {}", requirement_path.display(), e);
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }

    let success_count = succeeded.len() as u32;

    // Cleanup only successful requirements
    if !succeeded.is_empty() && !options.mock {
        for req_path in &succeeded {
            let path_str = req_path.to_string_lossy().to_string();
            match clean_requirement_apply_with_adapters(
                ExchangeCleanRequirementOptions { requirement_file: path_str },
                store,
                git,
            ) {
                Ok(cleanup_res) => {
                    eprintln!(
                        "Cleaned requirement {} ({} file(s) removed)",
                        cleanup_res.requirement_id,
                        cleanup_res.deleted_paths.len()
                    );
                }
                Err(e) => {
                    eprintln!("Failed to clean requirement {}: {}", req_path.display(), e);
                }
            }
        }

        // Single worker-branch publish after all cleanups
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

    // If all requirements failed, propagate the first error
    if success_count == 0
        && let Some(err) = first_error
    {
        return Err(err);
    }

    Ok(RunResults::with_count(success_count))
}
