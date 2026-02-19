use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkflowBootstrapCommands {
    /// Ensure/sync worker branch from target branch
    WorkerBranch,
    /// Materialize managed files from embedded scaffold
    ManagedFiles,
    /// Remove `.jules/exchange/changes.yml` for fresh narrator summary
    ExchangeChanges,
}

pub fn run_workflow_bootstrap(command: WorkflowBootstrapCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    let root = std::env::current_dir()
        .map_err(|e| AppError::InternalError(format!("Failed to get current directory: {}", e)))?;

    match command {
        WorkflowBootstrapCommands::WorkerBranch => {
            let options = workflow::WorkflowBootstrapWorkerBranchOptions { root };
            let output = workflow::bootstrap_worker_branch(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowBootstrapCommands::ManagedFiles => {
            let options = workflow::WorkflowBootstrapManagedFilesOptions { root };
            let output = workflow::bootstrap_managed_files(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowBootstrapCommands::ExchangeChanges => {
            let options = workflow::WorkflowBootstrapExchangeChangesOptions { root };
            let output = workflow::bootstrap_exchange_changes(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}
