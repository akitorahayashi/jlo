//! Workflow command implementation.

use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// Bootstrap the .jules/ runtime workspace on the current branch
    Bootstrap,
    /// Validation gate for .jules/ workspace
    Doctor,
    /// Export matrices for GitHub Actions
    Matrix {
        #[command(subcommand)]
        command: WorkflowMatrixCommands,
    },
    /// Run a layer and return wait-gating metadata
    Run {
        /// Target layer (narrator, observers, deciders, planners, implementers, innovators)
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

    /// Cleanup operations
    Cleanup {
        #[command(subcommand)]
        command: WorkflowCleanupCommands,
    },
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
    /// Inspect exchange and output JSON
    Inspect,
    /// Remove a processed issue and its source events
    CleanIssue {
        /// Path to the issue file
        issue_file: String,
    },
    /// Publish merged proposals as GitHub issues
    PublishProposals,
}

#[derive(Subcommand)]
pub enum WorkflowCleanupCommands {
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
    EnableAutomerge {
        /// PR number
        pr_number: u64,
    },
    /// Run all event-level PR commands in order
    Process {
        /// PR number
        pr_number: u64,
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

#[derive(Subcommand)]
pub enum WorkflowMatrixCommands {
    /// Check flat exchange for pending events
    Pending {
        /// Mock mode: always report pending events
        #[arg(long)]
        mock: bool,
    },
    /// Export planner/implementer issue matrices from flat exchange
    Routing {
        /// Routing labels as CSV (e.g., "bugs,feats,refacts,tests,docs")
        #[arg(long)]
        routing_labels: String,
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
        WorkflowCommands::Matrix { command } => run_workflow_matrix(command),
        WorkflowCommands::Run { layer, mock, phase } => {
            let layer = parse_layer(&layer)?;
            let mock_tag = std::env::var("JULES_MOCK_TAG").ok();
            let routing_labels = std::env::var("ROUTING_LABELS").ok().map(|s| {
                s.split(',').map(str::trim).filter(|v| !v.is_empty()).map(String::from).collect()
            });

            let options =
                workflow::WorkflowRunOptions { layer, mock, mock_tag, routing_labels, phase };
            let output = workflow::run(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Generate { mode, output_dir } => {
            let output_dir = output_dir.map(std::path::PathBuf::from);
            let options = workflow::WorkflowGenerateOptions { mode, output_dir };
            let output = workflow::generate(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Cleanup { command } => run_workflow_cleanup(command),
        WorkflowCommands::Pr { command } => run_workflow_pr(command),
        WorkflowCommands::Issue { command } => run_workflow_issue(command),
        WorkflowCommands::Inspect => {
            let options = workflow::WorkflowWorkstreamsInspectOptions {};
            let output = workflow::workstreams_inspect(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::CleanIssue { issue_file } => {
            let options = workflow::WorkflowWorkstreamsCleanIssueOptions { issue_file };
            let output = workflow::workstreams_clean_issue(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::PublishProposals => {
            let options = workflow::WorkflowWorkstreamsPublishProposalsOptions {};
            let output = workflow::workstreams_publish_proposals(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_cleanup(command: WorkflowCleanupCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowCleanupCommands::Mock { mock_tag, pr_numbers_json, branches_json } => {
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
            let options =
                workflow::WorkflowCleanupMockOptions { mock_tag, pr_numbers_json, branches_json };
            let output = workflow::cleanup_mock(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_pr(command: WorkflowPrCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    let github = crate::adapters::github_command::GitHubCommandAdapter::new();

    match command {
        WorkflowPrCommands::CommentSummaryRequest { pr_number } => {
            let options = workflow::pr::CommentSummaryRequestOptions { pr_number };
            let output = workflow::pr::events::comment_summary_request::execute(&github, options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowPrCommands::SyncCategoryLabel { pr_number } => {
            let options = workflow::pr::SyncCategoryLabelOptions { pr_number };
            let output = workflow::pr::events::sync_category_label::execute(&github, options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowPrCommands::EnableAutomerge { pr_number } => {
            let options = workflow::pr::EnableAutomergeOptions { pr_number };
            let output = workflow::pr::events::enable_automerge::execute(&github, options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowPrCommands::Process { pr_number } => {
            let options = workflow::pr::ProcessOptions { pr_number };
            let output = workflow::pr::process::execute(&github, options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_issue(command: WorkflowIssueCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    let github = crate::adapters::github_command::GitHubCommandAdapter::new();

    match command {
        WorkflowIssueCommands::LabelInnovator { issue_number, persona } => {
            let options = workflow::issue::LabelInnovatorOptions { issue_number, persona };
            let output = workflow::issue::label_innovator::execute(&github, options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_matrix(command: WorkflowMatrixCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow::{self, matrix};

    match command {
        WorkflowMatrixCommands::Pending { mock } => {
            let options = matrix::MatrixPendingOptions { mock };
            let output = matrix::pending(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowMatrixCommands::Routing { routing_labels } => {
            let options = matrix::MatrixRoutingOptions { routing_labels };
            let output = matrix::routing(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}
