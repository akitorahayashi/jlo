//! Workflow command implementation.

mod bootstrap;
mod process;
mod push;

use crate::domain::AppError;
use clap::Subcommand;

pub use bootstrap::WorkflowBootstrapCommands;
pub use process::WorkflowProcessCommands;
pub use push::WorkflowPushCommands;

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

    /// Process GitHub workflow actions
    Process {
        #[command(subcommand)]
        command: WorkflowProcessCommands,
    },

    /// Commit .jules changes and publish via worker branch
    Push {
        #[command(subcommand)]
        command: WorkflowPushCommands,
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

pub fn parse_layer(value: &str) -> Result<crate::domain::Layer, AppError> {
    crate::domain::Layer::from_dir_name(value)
        .ok_or_else(|| AppError::InvalidLayer { name: value.to_string() })
}

pub fn run_workflow(command: WorkflowCommands) -> Result<(), AppError> {
    match command {
        WorkflowCommands::Bootstrap { command } => bootstrap::run_workflow_bootstrap(command),
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
        WorkflowCommands::Process { command } => {
            let github = crate::adapters::github::GitHubCommandAdapter::new();
            process::run_workflow_process(&github, command)
        }
        WorkflowCommands::Push { command } => push::run_workflow_push(command),
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
