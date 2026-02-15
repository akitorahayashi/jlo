//! Workflow command implementation.

use crate::domain::AppError;
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// Bootstrap the .jules/ runtime workspace on the current branch
    Bootstrap,
    /// Validation gate for .jules/ workspace
    Doctor,
    /// Run a layer and return wait-gating metadata
    Run {
        /// Target layer (narrator, observers, decider, planner, implementer, integrator, innovators)
        layer: String,
        /// Run in mock mode (requires JULES_MOCK_TAG)
        #[arg(long)]
        mock: bool,
        /// Task selector for innovators (e.g. create_idea, refine_idea_and_create_proposal, create_proposal)
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

    /// GitHub entity operations (pr, issue)
    Gh {
        #[command(subcommand)]
        command: WorkflowGhCommands,
    },
    /// Exchange area observation and cleanup operations
    Exchange {
        #[command(subcommand)]
        command: WorkflowExchangeCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkflowExchangeCommands {
    /// Inspect exchange and output JSON
    Inspect,
    /// Publish merged proposals as GitHub issues
    PublishProposals,
    /// Clean exchange artifacts
    Clean {
        #[command(subcommand)]
        command: WorkflowExchangeCleanCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkflowExchangeCleanCommands {
    /// Remove a processed requirement and its source events
    Requirement {
        /// Path to the requirement file
        requirement_file: String,
    },
    /// Clean up mock artifacts
    Mock {
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
}

#[derive(Subcommand)]
pub enum WorkflowGhCommands {
    /// Process GitHub entities (pr, issue)
    Process {
        #[command(subcommand)]
        command: WorkflowProcessCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkflowProcessCommands {
    /// Process a Pull Request
    Pr {
        #[command(subcommand)]
        command: WorkflowProcessPrCommands,
    },
    /// Process an Issue
    Issue {
        #[command(subcommand)]
        command: WorkflowProcessIssueCommands,
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

#[derive(Subcommand)]
pub enum WorkflowProcessIssueCommands {
    /// Apply innovator labels to a proposal issue
    #[command(alias = "label-innovator")]
    LabelInnovator {
        /// Issue number
        issue_number: u64,
        /// Persona name (e.g., scout, architect)
        persona: String,
    },
}

pub fn parse_layer(value: &str) -> Result<crate::domain::Layer, AppError> {
    crate::domain::Layer::from_dir_name(value)
        .ok_or_else(|| AppError::InvalidLayer { name: value.to_string() })
}

pub fn run_workflow(command: WorkflowCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowCommands::Bootstrap => {
            let root = std::env::current_dir().map_err(|e| {
                AppError::InternalError(format!("Failed to get current directory: {}", e))
            })?;
            let options = workflow::WorkflowBootstrapOptions { root };
            let output = workflow::bootstrap(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Doctor => {
            let options = workflow::WorkflowDoctorOptions {};
            let output = workflow::doctor(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Run { layer, mock, task } => {
            let layer = parse_layer(&layer)?;
            let mock_tag = std::env::var("JULES_MOCK_TAG").ok();

            let options = workflow::WorkflowRunOptions { layer, mock, mock_tag, task };
            let output = workflow::run(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Generate { mode, output_dir } => {
            let output_dir = output_dir.map(std::path::PathBuf::from);
            let options = workflow::WorkflowGenerateOptions { mode, output_dir };
            let output = workflow::generate(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Gh { command } => run_workflow_gh(command),
        WorkflowCommands::Exchange { command } => run_workflow_exchange(command),
    }
}

fn run_workflow_exchange(command: WorkflowExchangeCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowExchangeCommands::Inspect => {
            let options = workflow::exchange::ExchangeInspectOptions {};
            let output = workflow::exchange::inspect(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowExchangeCommands::PublishProposals => {
            let options = workflow::exchange::ExchangePublishProposalsOptions {};
            let output = workflow::exchange::publish_proposals(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowExchangeCommands::Clean { command } => run_workflow_exchange_clean(command),
    }
}

fn run_workflow_exchange_clean(command: WorkflowExchangeCleanCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowExchangeCleanCommands::Requirement { requirement_file } => {
            let options = workflow::exchange::ExchangeCleanRequirementOptions { requirement_file };
            let output = workflow::exchange::clean_requirement(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowExchangeCleanCommands::Mock { mock_tag, pr_numbers_json, branches_json } => {
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
    }
}

fn run_workflow_gh(command: WorkflowGhCommands) -> Result<(), AppError> {
    let github = crate::adapters::github::GitHubCommandAdapter::new();
    match command {
        WorkflowGhCommands::Process { command } => match command {
            WorkflowProcessCommands::Pr { command } => run_workflow_gh_process_pr(&github, command),
            WorkflowProcessCommands::Issue { command } => {
                run_workflow_gh_process_issue(&github, command)
            }
        },
    }
}

fn run_workflow_gh_process_pr(
    github: &impl crate::ports::GitHubPort,
    command: WorkflowProcessPrCommands,
) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    let (pr_number, mode, args) = match command {
        WorkflowProcessPrCommands::All { pr_number, args } => {
            (pr_number, workflow::gh::pr::ProcessMode::All, args)
        }
        WorkflowProcessPrCommands::Metadata { pr_number, args } => {
            (pr_number, workflow::gh::pr::ProcessMode::Metadata, args)
        }
        WorkflowProcessPrCommands::Automerge { pr_number, args } => {
            (pr_number, workflow::gh::pr::ProcessMode::Automerge, args)
        }
    };
    let options = workflow::gh::pr::ProcessOptions {
        pr_number,
        mode,
        fail_on_error: args.fail_on_error,
        retry_attempts: args.retry_attempts,
        retry_delay_seconds: args.retry_delay_seconds,
    };
    let output = workflow::gh::pr::process::execute(github, options)?;
    workflow::write_workflow_output(&output)
}

fn run_workflow_gh_process_issue(
    github: &impl crate::ports::GitHubPort,
    command: WorkflowProcessIssueCommands,
) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowProcessIssueCommands::LabelInnovator { issue_number, persona } => {
            let options = workflow::gh::issue::LabelInnovatorOptions { issue_number, persona };
            let output = workflow::gh::issue::label_innovator::execute(github, options)?;
            workflow::write_workflow_output(&output)
        }
    }
}
