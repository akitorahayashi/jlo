use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkflowPushCommands {
    /// Commit .jules changes, create PR to worker branch, and merge it
    WorkerBranch {
        /// Stable token used in branch naming (e.g. requirement-cleanup)
        #[arg(long)]
        change_token: String,
        /// Commit message
        #[arg(long)]
        commit_message: String,
        /// Pull request title
        #[arg(long)]
        pr_title: String,
        /// Pull request body
        #[arg(long)]
        pr_body: String,
    },
}

pub fn run_workflow_push(command: WorkflowPushCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowPushCommands::WorkerBranch { change_token, commit_message, pr_title, pr_body } => {
            let output = workflow::push::execute(workflow::push::PushWorkerBranchOptions {
                change_token,
                commit_message,
                pr_title,
                pr_body,
            })?;
            workflow::write_workflow_output(&output)
        }
    }
}
