use crate::domain::AppError;
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum WorkflowProcessCommands {
    /// Process Pull Request workflow actions
    Pr {
        #[command(subcommand)]
        command: WorkflowProcessPrCommands,
    },
    /// Process Issue workflow actions
    Issue {
        #[command(subcommand)]
        command: WorkflowProcessIssueCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkflowProcessIssueCommands {
    /// Apply innovator labels to a proposal issue
    LabelInnovator {
        /// Issue number
        issue_number: u64,
        /// Role name (e.g., scout, architect)
        role: String,
    },
}

/// Shared flags for PR processing commands.
#[derive(Args, Debug, Clone)]
pub struct ProcessPrArgs {
    /// Fail if any step returns an execution error
    #[arg(long)]
    pub fail_on_error: bool,
    /// Retry attempts for transient auto-merge errors
    #[arg(long, default_value_t = 1)]
    pub retry_attempts: u32,
    /// Delay between retry attempts (seconds)
    #[arg(long, default_value_t = 0)]
    pub retry_delay_seconds: u64,
}

#[derive(Subcommand)]
pub enum WorkflowProcessPrCommands {
    /// Run all PR event commands
    All {
        /// PR number
        pr_number: u64,
        #[command(flatten)]
        args: ProcessPrArgs,
    },
    /// Run metadata-only commands
    Metadata {
        /// PR number
        pr_number: u64,
        #[command(flatten)]
        args: ProcessPrArgs,
    },
    /// Run auto-merge command only
    #[command(alias = "auto-merge")]
    Automerge {
        /// PR number
        pr_number: u64,
        #[command(flatten)]
        args: ProcessPrArgs,
    },
}

pub fn run_workflow_process(
    github: &impl crate::ports::GitHub,
    command: WorkflowProcessCommands,
) -> Result<(), AppError> {
    match command {
        WorkflowProcessCommands::Pr { command } => run_workflow_process_pr(github, command),
        WorkflowProcessCommands::Issue { command } => run_workflow_process_issue(github, command),
    }
}

fn run_workflow_process_issue(
    github: &impl crate::ports::GitHub,
    command: WorkflowProcessIssueCommands,
) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowProcessIssueCommands::LabelInnovator { issue_number, role } => {
            let options = workflow::process::issue::LabelInnovatorOptions { issue_number, role };
            let output = workflow::process::issue::label_innovator::execute(github, options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_process_pr(
    github: &impl crate::ports::GitHub,
    command: WorkflowProcessPrCommands,
) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    let (pr_number, mode, args) = match command {
        WorkflowProcessPrCommands::All { pr_number, args } => {
            (pr_number, workflow::process::pr::ProcessMode::All, args)
        }
        WorkflowProcessPrCommands::Metadata { pr_number, args } => {
            (pr_number, workflow::process::pr::ProcessMode::Metadata, args)
        }
        WorkflowProcessPrCommands::Automerge { pr_number, args } => {
            (pr_number, workflow::process::pr::ProcessMode::Automerge, args)
        }
    };
    let options = workflow::process::pr::ProcessOptions {
        pr_number,
        mode,
        fail_on_error: args.fail_on_error,
        retry_attempts: args.retry_attempts,
        retry_delay_seconds: args.retry_delay_seconds,
    };
    let output = workflow::process::pr::process::execute(github, options)?;
    workflow::write_workflow_output(&output)
}
