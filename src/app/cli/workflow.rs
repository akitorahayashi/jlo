//! Workflow command implementation.

use crate::domain::AppError;
use clap::Subcommand;

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
        /// Execution phase for innovators (creation or refinement)
        #[arg(long)]
        phase: Option<String>,
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
    /// Workspace observation and cleanup operations
    Workspace {
        #[command(subcommand)]
        command: WorkflowWorkspaceCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkflowWorkspaceCommands {
    /// Inspect exchange and output JSON
    Inspect,
    /// Publish merged proposals as GitHub issues
    PublishProposals,
    /// Clean workspace artifacts
    Clean {
        #[command(subcommand)]
        command: WorkflowWorkspaceCleanCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkflowWorkspaceCleanCommands {
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
    /// PR operations
    Pr {
        #[command(subcommand)]
        command: WorkflowPrCommands,
    },
    /// Issue operations
    Issue {
        #[command(subcommand)]
        command: WorkflowIssueCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkflowPrCommands {
    /// Post or update the summary-request comment on a Jules PR
    CommentSummaryRequest {
        /// PR number
        pr_number: u64,
    },
    /// Sync implementer category label from branch to PR
    SyncCategoryLabel {
        /// PR number
        pr_number: u64,
    },
    /// Enable auto-merge on an eligible PR
    #[command(alias = "enable-automerge")]
    Automerge {
        /// PR number
        pr_number: u64,
    },
    /// Run PR event commands in configured mode
    Process {
        /// PR number
        pr_number: u64,
        /// Execution mode (all, metadata, automerge)
        #[arg(long, default_value = "all", value_parser = ["all", "metadata", "automerge"])]
        mode: String,
        /// Fail if any step returns an execution error
        #[arg(long)]
        fail_on_error: bool,
        /// Retry attempts for transient auto-merge errors
        #[arg(long, default_value_t = 1)]
        retry_attempts: u32,
        /// Delay between retry attempts (seconds)
        #[arg(long, default_value_t = 0)]
        retry_delay_seconds: u64,
    },
}

#[derive(Subcommand)]
pub enum WorkflowIssueCommands {
    /// Apply innovator labels to a proposal issue
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
        WorkflowCommands::Run { layer, mock, phase } => {
            let layer = parse_layer(&layer)?;
            let mock_tag = std::env::var("JULES_MOCK_TAG").ok();

            let options = workflow::WorkflowRunOptions { layer, mock, mock_tag, phase };
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
        WorkflowCommands::Workspace { command } => run_workflow_workspace(command),
    }
}

fn run_workflow_workspace(command: WorkflowWorkspaceCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowWorkspaceCommands::Inspect => {
            let options = workflow::workspace::WorkspaceInspectOptions {};
            let output = workflow::workspace::inspect(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowWorkspaceCommands::PublishProposals => {
            let options = workflow::workspace::WorkspacePublishProposalsOptions {};
            let output = workflow::workspace::publish_proposals(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowWorkspaceCommands::Clean { command } => run_workflow_workspace_clean(command),
    }
}

fn run_workflow_workspace_clean(command: WorkflowWorkspaceCleanCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowWorkspaceCleanCommands::Requirement { requirement_file } => {
            let options =
                workflow::workspace::WorkspaceCleanRequirementOptions { requirement_file };
            let output = workflow::workspace::clean_requirement(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowWorkspaceCleanCommands::Mock { mock_tag, pr_numbers_json, branches_json } => {
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
            let options = workflow::workspace::WorkspaceCleanMockOptions {
                mock_tag,
                pr_numbers_json,
                branches_json,
            };
            let output = workflow::workspace::clean_mock(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_gh(command: WorkflowGhCommands) -> Result<(), AppError> {
    let github = crate::adapters::github_command::GitHubCommandAdapter::new();
    match command {
        WorkflowGhCommands::Pr { command } => run_workflow_gh_pr(&github, command),
        WorkflowGhCommands::Issue { command } => run_workflow_gh_issue(&github, command),
    }
}

fn run_workflow_gh_pr(
    github: &impl crate::ports::GitHubPort,
    command: WorkflowPrCommands,
) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowPrCommands::CommentSummaryRequest { pr_number } => {
            let options = workflow::gh::pr::CommentSummaryRequestOptions { pr_number };
            let output =
                workflow::gh::pr::events::comment_summary_request::execute(github, options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowPrCommands::SyncCategoryLabel { pr_number } => {
            let options = workflow::gh::pr::SyncCategoryLabelOptions { pr_number };
            let output = workflow::gh::pr::events::sync_category_label::execute(github, options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowPrCommands::Automerge { pr_number } => {
            let options = workflow::gh::pr::EnableAutomergeOptions { pr_number };
            let output = workflow::gh::pr::events::enable_automerge::execute(github, options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowPrCommands::Process {
            pr_number,
            mode,
            fail_on_error,
            retry_attempts,
            retry_delay_seconds,
        } => {
            let mode = match mode.as_str() {
                "all" => workflow::gh::pr::ProcessMode::All,
                "metadata" => workflow::gh::pr::ProcessMode::Metadata,
                "automerge" => workflow::gh::pr::ProcessMode::Automerge,
                _ => {
                    return Err(AppError::Validation(format!(
                        "Invalid process mode '{}'. Expected one of: all, metadata, automerge",
                        mode
                    )));
                }
            };
            let options = workflow::gh::pr::ProcessOptions {
                pr_number,
                mode,
                fail_on_error,
                retry_attempts,
                retry_delay_seconds,
            };
            let output = workflow::gh::pr::process::execute(github, options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_gh_issue(
    github: &impl crate::ports::GitHubPort,
    command: WorkflowIssueCommands,
) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowIssueCommands::LabelInnovator { issue_number, persona } => {
            let options = workflow::gh::issue::LabelInnovatorOptions { issue_number, persona };
            let output = workflow::gh::issue::label_innovator::execute(github, options)?;
            workflow::write_workflow_output(&output)
        }
    }
}
