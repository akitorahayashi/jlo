//! Workflow command implementation.

use crate::domain::AppError;
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// Bootstrap the .jules/ runtime repository on the current branch
    Bootstrap {
        #[command(subcommand)]
        command: WorkflowBootstrapCommands,
    },
    /// Validation gate for .jules/ repository
    Doctor,
    /// Run a layer and return wait-gating metadata
    Run {
        /// Target layer (narrator, observers, decider, planner, implementer, integrator, innovators)
        layer: String,
        /// Run in mock mode (requires JULES_MOCK_TAG)
        #[arg(long)]
        mock: bool,
        /// Override starting branch for Jules API request
        #[arg(long)]
        branch: Option<String>,
        /// Task selector for innovators (expected: create_three_proposals)
        #[arg(long)]
        task: Option<String>,
    },
    /// Generate workflow scaffold files to an output directory
    #[clap(visible_alias = "g")]
    Generate {
        /// Runner mode (remote or self-hosted)
        mode: crate::domain::WorkflowRunnerMode,
        /// Output directory override (default: repository .github/)
        #[arg(short = 'o', long = "output-dir")]
        output_dir: Option<String>,
    },

    /// Process a Pull Request
    ProcessPr {
        #[command(subcommand)]
        command: WorkflowProcessPrCommands,
    },

    /// Apply innovator labels to a proposal issue
    LabelInnovator {
        /// Issue number
        issue_number: u64,
        /// Role name (e.g., scout, architect)
        role: String,
    },

    /// Commit .jules changes, create PR to worker branch, and merge it
    PushWorker {
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

    /// Remove a processed requirement and its source events
    CleanRequirement {
        /// Path to the requirement file
        requirement_file: String,
    },

    /// Clean up mock artifacts
    CleanMock {
        /// Mock tag to identify artifacts
        #[arg(long)]
        mock_tag: String,
        /// PR numbers JSON array to close
        #[arg(long)]
        pr_numbers_json: Option<String>,
        /// Branches JSON array to delete
        #[arg(long)]
        branches_json: Option<String>,
    },

    /// Inspect exchange and output JSON
    InspectExchange,

    /// Publish merged proposals as GitHub issues
    PublishProposals,
}

#[derive(Subcommand)]
pub enum WorkflowBootstrapCommands {
    /// Ensure/sync worker branch from target branch
    WorkerBranch,
    /// Materialize managed files from embedded scaffold
    ManagedFiles,
    /// Reconcile workstation perspectives from schedule intent
    Workstations,
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

pub fn parse_layer(value: &str) -> Result<crate::domain::Layer, AppError> {
    crate::domain::Layer::from_dir_name(value)
        .ok_or_else(|| AppError::InvalidLayer { name: value.to_string() })
}

pub fn run_workflow(command: WorkflowCommands) -> Result<(), AppError> {
    match command {
        WorkflowCommands::Bootstrap { command } => run_workflow_bootstrap(command),
        WorkflowCommands::Doctor => {
            use crate::app::commands::workflow;
            let options = workflow::WorkflowDoctorOptions {};
            let output = workflow::doctor(options)?;
            workflow::write_workflow_output(&output)?;
            if !output.ok {
                return Err(AppError::Validation("Workflow doctor checks failed".to_string()));
            }
            Ok(())
        }
        WorkflowCommands::Run { layer, mock, branch, task } => {
            use crate::app::commands::workflow;
            let layer = parse_layer(&layer)?;
            let mock_tag = std::env::var("JULES_MOCK_TAG").ok();

            let options = workflow::WorkflowRunOptions { layer, mock, branch, mock_tag, task };
            let output = workflow::run(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Generate { mode, output_dir } => {
            use crate::app::commands::workflow;
            let output_dir = output_dir.map(std::path::PathBuf::from);
            let options = workflow::WorkflowGenerateOptions { mode, output_dir };
            let output = workflow::generate(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::ProcessPr { command } => {
            let github = crate::adapters::github::GitHubCommandAdapter::new();
            run_workflow_gh_process_pr(&github, command)
        }
        WorkflowCommands::LabelInnovator { issue_number, role } => {
            let github = crate::adapters::github::GitHubCommandAdapter::new();
            use crate::app::commands::workflow;
            let options = workflow::gh::process::issue::LabelInnovatorOptions { issue_number, role };
            let output = workflow::gh::process::issue::label_innovator::execute(&github, options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::PushWorker { change_token, commit_message, pr_title, pr_body } => {
            use crate::app::commands::workflow;
            let output =
                workflow::gh::push::execute(workflow::gh::push::PushWorkerBranchOptions {
                    change_token,
                    commit_message,
                    pr_title,
                    pr_body,
                })?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::CleanRequirement { requirement_file } => {
            use crate::app::commands::workflow;
            let options = workflow::exchange::ExchangeCleanRequirementOptions { requirement_file };
            let output = workflow::exchange::clean_requirement(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::CleanMock { mock_tag, pr_numbers_json, branches_json } => {
            use crate::app::commands::workflow;
            let pr_numbers_json = match pr_numbers_json {
                Some(json_str) => {
                    let parsed: Vec<u64> = serde_json::from_str(&json_str).map_err(|e| {
                        AppError::Validation(format!("Invalid pr-numbers-json: {}", e))
                    })?;
                    Some(parsed)
                }
                None => None,
            };
            let branches_json = match branches_json {
                Some(json_str) => {
                    let parsed: Vec<String> = serde_json::from_str(&json_str).map_err(|e| {
                        AppError::Validation(format!("Invalid branches-json: {}", e))
                    })?;
                    Some(parsed)
                }
                None => None,
            };
            let options = workflow::exchange::ExchangeCleanMockOptions {
                mock_tag,
                pr_numbers_json,
                branches_json,
            };
            let output = workflow::exchange::clean_mock(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::InspectExchange => {
            use crate::app::commands::workflow;
            let options = workflow::exchange::ExchangeInspectOptions {};
            let output = workflow::exchange::inspect(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::PublishProposals => {
            use crate::app::commands::workflow;
            let options = workflow::exchange::ExchangePublishProposalsOptions {};
            let output = workflow::exchange::publish_proposals(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_bootstrap(command: WorkflowBootstrapCommands) -> Result<(), AppError> {
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
        WorkflowBootstrapCommands::Workstations => {
            let options = workflow::WorkflowBootstrapWorkstationsOptions { root };
            let output = workflow::bootstrap_workstations(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_gh_process_pr(
    github: &impl crate::ports::GitHub,
    command: WorkflowProcessPrCommands,
) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    let (pr_number, mode, args) = match command {
        WorkflowProcessPrCommands::All { pr_number, args } => {
            (pr_number, workflow::gh::process::pr::ProcessMode::All, args)
        }
        WorkflowProcessPrCommands::Metadata { pr_number, args } => {
            (pr_number, workflow::gh::process::pr::ProcessMode::Metadata, args)
        }
        WorkflowProcessPrCommands::Automerge { pr_number, args } => {
            (pr_number, workflow::gh::process::pr::ProcessMode::Automerge, args)
        }
    };
    let options = workflow::gh::process::pr::ProcessOptions {
        pr_number,
        mode,
        fail_on_error: args.fail_on_error,
        retry_attempts: args.retry_attempts,
        retry_delay_seconds: args.retry_delay_seconds,
    };
    let output = workflow::gh::process::pr::process::execute(github, options)?;
    workflow::write_workflow_output(&output)
}
