//! CLI Adapter.

use std::path::PathBuf;

use crate::domain::AppError;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jlo")]
#[command(version)]
#[command(
    about = "Deploy and manage .jules/ workspace scaffolding",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create .jules/ workspace structure
    #[clap(visible_alias = "i")]
    Init {
        #[command(subcommand)]
        command: Option<InitCommands>,
    },
    /// Update .jules/ workspace to current jlo version
    #[clap(visible_alias = "u")]
    Update {
        /// Show planned changes without applying
        #[arg(long)]
        prompt_preview: bool,
        /// Adopt current default role files as managed baseline (skips conditional updates)
        #[arg(long)]
        adopt_managed: bool,
    },
    /// Apply a template (workstream or role)
    #[clap(visible_alias = "tp")]
    Template {
        /// Layer: observers or deciders (multi-role layers only)
        #[arg(short, long)]
        layer: Option<String>,
        /// Name for the new role (blank role only)
        #[arg(short, long)]
        name: Option<String>,
        /// Target workstream for observers/deciders
        #[arg(short, long)]
        workstream: Option<String>,
    },
    /// Setup compiler commands
    #[clap(visible_alias = "s")]
    Setup {
        #[command(subcommand)]
        command: SetupCommands,
    },
    /// Execute Jules agents
    #[clap(visible_alias = "r")]
    Run {
        #[command(subcommand)]
        layer: RunLayer,
    },
    /// Workflow orchestration primitives for GitHub Actions
    #[clap(visible_alias = "wf")]
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    /// Inspect workstreams for automation
    Workstreams {
        #[command(subcommand)]
        command: WorkstreamCommands,
    },
    /// Validate .jules/ structure and content
    Doctor {
        /// Attempt to auto-fix recoverable issues
        #[arg(long)]
        fix: bool,
        /// Treat warnings as failures
        #[arg(long)]
        strict: bool,
        /// Limit checks to a specific workstream
        #[arg(long)]
        workstream: Option<String>,
    },
    /// Remove jlo-managed assets (branch + workflows)
    Deinit,
}

#[derive(Subcommand)]
enum SetupCommands {
    /// Generate install.sh and env.toml from tools.yml
    #[clap(visible_alias = "g")]
    Gen {
        /// Project directory containing .jules/setup/ (defaults to current directory)
        path: Option<PathBuf>,
    },
    /// List available components
    #[clap(visible_alias = "ls")]
    List {
        /// Show detailed info for a specific component
        #[arg(long)]
        detail: Option<String>,
    },
}

#[derive(Subcommand)]
enum InitCommands {
    /// Create .jules/ workspace structure
    #[clap(visible_alias = "s")]
    Scaffold,
    /// Install GitHub Actions workflow kit
    #[clap(visible_alias = "w")]
    Workflows {
        /// Install the GitHub-hosted runner workflow kit
        #[arg(
            short = 'r',
            long,
            conflicts_with = "self_hosted",
            required_unless_present = "self_hosted"
        )]
        remote: bool,
        /// Install the self-hosted runner workflow kit
        #[arg(short = 's', long, conflicts_with = "remote", required_unless_present = "remote")]
        self_hosted: bool,
    },
}

#[derive(Subcommand)]
enum RunLayer {
    /// Run narrator agent (summarizes codebase changes)
    #[clap(visible_alias = "n")]
    Narrator {
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run observer agents
    #[clap(visible_alias = "o")]
    Observers {
        /// Specific roles to run (manual mode)
        #[arg(short = 'r', long)]
        role: Option<Vec<String>>,
        /// Target workstream
        #[arg(short = 'w', long)]
        workstream: Option<String>,
        /// Run using scheduled.toml roles
        #[arg(long)]
        scheduled: bool,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run decider agents
    #[clap(visible_alias = "d")]
    Deciders {
        /// Specific roles to run (manual mode)
        #[arg(short = 'r', long)]
        role: Option<Vec<String>>,
        /// Target workstream
        #[arg(short = 'w', long)]
        workstream: Option<String>,
        /// Run using scheduled.toml roles
        #[arg(long)]
        scheduled: bool,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run planner agent (single-role, issue-driven)
    #[clap(visible_alias = "p")]
    Planners {
        /// Local issue file path (required)
        issue: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run implementer agent (single-role, issue-driven)
    #[clap(visible_alias = "i")]
    Implementers {
        /// Local issue file path (required)
        issue: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Validation gate for .jules/ workspace
    Doctor {
        /// Limit checks to a specific workstream
        #[arg(long)]
        workstream: Option<String>,
    },
    /// Export matrices for GitHub Actions
    Matrix {
        #[command(subcommand)]
        command: WorkflowMatrixCommands,
    },
    /// Run a layer sequentially and return wait-gating metadata
    Run {
        /// Target layer
        #[arg(long)]
        layer: String,
        /// Matrix JSON input (from matrix commands)
        #[arg(long)]
        matrix_json: Option<String>,
        /// Target branch for implementers
        #[arg(long)]
        target_branch: Option<String>,
        /// Run in mock mode (requires JULES_MOCK_TAG)
        #[arg(long)]
        mock: bool,
    },
    /// Wait for PR readiness conditions
    Wait {
        #[command(subcommand)]
        command: WorkflowWaitCommands,
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
}

#[derive(Subcommand)]
enum WorkflowWaitCommands {
    /// Wait for PRs to be ready (time-based wait)
    Prs {
        /// Target layer (used to resolve branch_prefix)
        #[arg(long)]
        layer: String,
        /// Base branch for PR discovery
        #[arg(long)]
        base_branch: String,
        /// Run started timestamp (RFC3339 UTC)
        #[arg(long)]
        run_started_at: String,
        /// Maximum wait time in minutes
        #[arg(long)]
        wait_minutes: u32,
        /// Wait mode: merge or label
        #[arg(long)]
        mode: String,
        /// Mock mode (overrides timeout to 30 seconds)
        #[arg(long)]
        mock: bool,
        /// Mock PR numbers JSON array (bypasses PR discovery)
        #[arg(long)]
        mock_pr_numbers_json: Option<String>,
    },
}

#[derive(Subcommand)]
enum WorkflowCleanupCommands {
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
    /// Clean up a processed issue and its source events
    ProcessedIssue {
        /// Path to the issue file
        #[arg(long)]
        issue_file: String,
        /// Commit changes (default: true)
        #[arg(long, default_value = "true")]
        commit: bool,
        /// Push changes (default: true)
        #[arg(long, default_value = "true")]
        push: bool,
    },
}

#[derive(Subcommand)]
enum WorkflowPrCommands {
    /// Apply category label to implementer PR from branch name
    LabelFromBranch {
        /// Branch name (defaults to GITHUB_REF_NAME)
        #[arg(long)]
        branch: Option<String>,
    },
}

#[derive(Subcommand)]
enum WorkflowMatrixCommands {
    /// Export enabled workstreams as a GitHub Actions matrix
    Workstreams,
    /// Export enabled roles for a multi-role layer as a GitHub Actions matrix
    Roles {
        /// Target layer (observers or deciders)
        #[arg(long)]
        layer: String,
        /// Workstreams JSON from `matrix workstreams` output (the `matrix` field)
        #[arg(long)]
        workstreams_json: String,
    },
    /// Export workstreams with pending events as a GitHub Actions matrix
    PendingWorkstreams {
        /// Workstreams JSON from `matrix workstreams` output (the `matrix` field)
        #[arg(long)]
        workstreams_json: String,
        /// Mock mode: treat all workstreams as having pending events
        #[arg(long)]
        mock: bool,
    },
    /// Export planner/implementer issue matrices from workstream inspection
    Routing {
        /// Workstreams JSON from `matrix workstreams` output (the `matrix` field)
        #[arg(long)]
        workstreams_json: String,
        /// Routing labels as CSV (e.g., "bugs,feats,refacts,tests,docs")
        #[arg(long)]
        routing_labels: String,
    },
}

#[derive(Subcommand)]
enum WorkstreamCommands {
    /// Inspect a workstream and output JSON/YAML
    Inspect {
        /// Workstream name
        #[arg(long)]
        workstream: String,
        /// Output format (json or yaml)
        #[arg(long, default_value = "json")]
        format: String,
    },
}

/// Entry point for the CLI.
pub fn run() {
    let cli = Cli::parse();

    let result: Result<i32, AppError> = match cli.command {
        Commands::Init { command } => run_init(command).map(|_| 0),
        Commands::Update { prompt_preview, adopt_managed } => {
            run_update(prompt_preview, adopt_managed).map(|_| 0)
        }
        Commands::Template { layer, name, workstream } => {
            run_template(layer, name, workstream).map(|_| 0)
        }
        Commands::Setup { command } => match command {
            SetupCommands::Gen { path } => run_setup_gen(path).map(|_| 0),
            SetupCommands::List { detail } => run_setup_list(detail).map(|_| 0),
        },
        Commands::Run { layer } => run_agents(layer).map(|_| 0),
        Commands::Workflow { command } => run_workflow(command).map(|_| 0),
        Commands::Workstreams { command } => run_workstreams(command).map(|_| 0),
        Commands::Doctor { fix, strict, workstream } => run_doctor(fix, strict, workstream),
        Commands::Deinit => run_deinit().map(|_| 0),
    };

    match result {
        Ok(exit_code) => {
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_init(command: Option<InitCommands>) -> Result<(), AppError> {
    match command.unwrap_or(InitCommands::Scaffold) {
        InitCommands::Scaffold => {
            crate::app::api::init()?;
            println!("✅ Initialized .jules/ workspace");
            Ok(())
        }
        InitCommands::Workflows { remote, self_hosted } => {
            let mode = if remote {
                crate::domain::WorkflowRunnerMode::Remote
            } else if self_hosted {
                crate::domain::WorkflowRunnerMode::SelfHosted
            } else {
                return Err(AppError::MissingArgument(
                    "Runner mode is required. Use --remote or --self-hosted.".into(),
                ));
            };
            crate::app::api::init_workflows(mode)?;
            println!("✅ Installed workflow kit ({})", mode.label());
            Ok(())
        }
    }
}

fn run_template(
    layer: Option<String>,
    name: Option<String>,
    workstream: Option<String>,
) -> Result<(), AppError> {
    let outcome =
        crate::app::api::template(layer.as_deref(), name.as_deref(), workstream.as_deref())?;

    let entity_type = match &outcome {
        crate::app::api::TemplateOutcome::Role { .. } => "role",
        crate::app::api::TemplateOutcome::Workstream { .. } => "workstream",
    };
    println!("✅ Created new {} at {}/", entity_type, outcome.display_path());
    Ok(())
}

fn run_update(prompt_preview: bool, adopt_managed: bool) -> Result<(), AppError> {
    let result = crate::app::api::update(prompt_preview, adopt_managed)?;

    if !result.prompt_preview {
        if result.updated.is_empty() && result.created.is_empty() && result.removed.is_empty() {
            println!("✅ Workspace already up to date");
            if result.adopted_managed {
                println!("  Managed baseline recorded for default role files");
            }
            if !result.skipped.is_empty() {
                println!("  Skipped {} file(s):", result.skipped.len());
                for skipped in &result.skipped {
                    println!("    - {} ({})", skipped.path, skipped.reason);
                }
            }
        } else {
            println!("✅ Updated workspace to version {}", env!("CARGO_PKG_VERSION"));
            if !result.updated.is_empty() {
                println!("  Updated {} file(s)", result.updated.len());
            }
            if !result.created.is_empty() {
                println!("  Created {} file(s)", result.created.len());
            }
            if !result.removed.is_empty() {
                println!("  Removed {} file(s)", result.removed.len());
            }
            if !result.skipped.is_empty() {
                println!("  Skipped {} file(s):", result.skipped.len());
                for skipped in &result.skipped {
                    println!("    - {} ({})", skipped.path, skipped.reason);
                }
            }
            if result.adopted_managed {
                println!("  Managed baseline recorded for default role files");
            }
            if let Some(backup) = result.backup_path {
                println!("  Backup at: {}", backup.display());
            }
        }
    }

    Ok(())
}

fn run_agents(layer: RunLayer) -> Result<(), AppError> {
    use crate::domain::Layer;

    let (target_layer, roles, workstream, scheduled, prompt_preview, branch, issue, mock) =
        match layer {
            RunLayer::Narrator { prompt_preview, branch, mock } => {
                (Layer::Narrators, None, None, false, prompt_preview, branch, None, mock)
            }
            RunLayer::Observers { role, prompt_preview, branch, workstream, scheduled, mock } => {
                (Layer::Observers, role, workstream, scheduled, prompt_preview, branch, None, mock)
            }
            RunLayer::Deciders { role, prompt_preview, branch, workstream, scheduled, mock } => {
                (Layer::Deciders, role, workstream, scheduled, prompt_preview, branch, None, mock)
            }
            RunLayer::Planners { prompt_preview, branch, issue, mock } => {
                (Layer::Planners, None, None, false, prompt_preview, branch, Some(issue), mock)
            }
            RunLayer::Implementers { prompt_preview, branch, issue, mock } => {
                (Layer::Implementers, None, None, false, prompt_preview, branch, Some(issue), mock)
            }
        };

    let result = crate::app::api::run(
        target_layer,
        roles,
        workstream,
        scheduled,
        prompt_preview,
        branch,
        issue,
        mock,
    )?;

    if !result.prompt_preview && !result.roles.is_empty() && !result.sessions.is_empty() {
        println!("✅ Created {} Jules session(s)", result.sessions.len());
    }

    Ok(())
}

fn run_setup_gen(path: Option<PathBuf>) -> Result<(), AppError> {
    let components = crate::app::api::setup_gen(path.as_deref())?;
    println!("✅ Generated install.sh with {} component(s)", components.len());
    for (i, name) in components.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    Ok(())
}

fn run_setup_list(detail: Option<String>) -> Result<(), AppError> {
    if let Some(component) = detail {
        let info = crate::app::api::setup_detail(&component)?;
        println!("{}: {}", info.name, info.summary);
        if !info.dependencies.is_empty() {
            println!("\nDependencies:");
            for dep in &info.dependencies {
                println!("  • {}", dep);
            }
        }
        if !info.env_vars.is_empty() {
            println!("\nEnvironment Variables:");
            for env in &info.env_vars {
                let default_str =
                    env.default.as_ref().map(|d| format!(" (default: {})", d)).unwrap_or_default();
                println!("  • {}{}", env.name, default_str);
                if !env.description.is_empty() {
                    println!("    {}", env.description);
                }
            }
        }
        println!("\nInstall Script:");
        println!("{}", info.script_content);
    } else {
        let components = crate::app::api::setup_list()?;
        println!("Available components:");
        for comp in components {
            println!("  {} - {}", comp.name, comp.summary);
        }
    }
    Ok(())
}

fn run_deinit() -> Result<(), AppError> {
    let outcome = crate::app::api::deinit()?;

    if outcome.deleted_branch {
        println!("✅ Deleted local 'jules' branch");
    } else {
        println!("ℹ️ Local 'jules' branch not found");
    }

    if outcome.deleted_files.is_empty() && outcome.deleted_action_dirs.is_empty() {
        println!("ℹ️ No workflow kit files found to remove");
    } else {
        if !outcome.deleted_files.is_empty() {
            println!("✅ Removed {} workflow kit file(s)", outcome.deleted_files.len());
        }
        if !outcome.deleted_action_dirs.is_empty() {
            println!(
                "✅ Removed {} workflow action directory(ies)",
                outcome.deleted_action_dirs.len()
            );
        }
    }

    println!("⚠️ Remove JULES_API_KEY and JULES_API_SECRET from GitHub repository settings.");
    Ok(())
}

fn run_workstreams(command: WorkstreamCommands) -> Result<(), AppError> {
    match command {
        WorkstreamCommands::Inspect { workstream, format } => {
            let format = parse_workstream_format(&format)?;
            let output = crate::app::api::workstreams_inspect(crate::WorkstreamInspectOptions {
                workstream,
                format: format.clone(),
            })?;

            match format {
                crate::WorkstreamInspectFormat::Json => {
                    print_json(&output)?;
                }
                crate::WorkstreamInspectFormat::Yaml => {
                    print_yaml(&output)?;
                }
            }
            Ok(())
        }
    }
}

fn parse_workstream_format(format: &str) -> Result<crate::WorkstreamInspectFormat, AppError> {
    match format {
        "json" => Ok(crate::WorkstreamInspectFormat::Json),
        "yaml" => Ok(crate::WorkstreamInspectFormat::Yaml),
        _ => Err(AppError::Validation("Invalid workstream inspect format".into())),
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|err| AppError::InternalError(format!("Failed to serialize output: {}", err)))?;
    println!("{}", json);
    Ok(())
}

fn print_yaml<T: serde::Serialize>(value: &T) -> Result<(), AppError> {
    let yaml = serde_yaml::to_string(value)
        .map_err(|err| AppError::InternalError(format!("Failed to serialize output: {}", err)))?;
    println!("{}", yaml.trim_end());
    Ok(())
}

fn parse_layer(value: &str) -> Result<crate::domain::Layer, AppError> {
    crate::domain::Layer::from_dir_name(value)
        .ok_or_else(|| AppError::InvalidLayer { name: value.to_string() })
}

fn run_doctor(fix: bool, strict: bool, workstream: Option<String>) -> Result<i32, AppError> {
    let options = crate::DoctorOptions { fix, strict, workstream };
    let outcome = crate::app::api::doctor(options)?;

    Ok(outcome.exit_code)
}

fn run_workflow(command: WorkflowCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowCommands::Doctor { workstream } => {
            let options = workflow::WorkflowDoctorOptions { workstream };
            let output = workflow::doctor(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Matrix { command } => run_workflow_matrix(command),
        WorkflowCommands::Run { layer, matrix_json, target_branch, mock } => {
            let layer = parse_layer(&layer)?;
            let matrix_json = match matrix_json {
                Some(json_str) => {
                    let parsed: serde_json::Value = serde_json::from_str(&json_str)
                        .map_err(|e| AppError::Validation(format!("Invalid matrix-json: {}", e)))?;
                    Some(parsed)
                }
                None => None,
            };
            let options = workflow::WorkflowRunOptions { layer, matrix_json, target_branch, mock };
            let output = workflow::run(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowCommands::Wait { command } => run_workflow_wait(command),
        WorkflowCommands::Cleanup { command } => run_workflow_cleanup(command),
        WorkflowCommands::Pr { command } => run_workflow_pr(command),
    }
}

fn run_workflow_wait(command: WorkflowWaitCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowWaitCommands::Prs {
            layer,
            base_branch,
            run_started_at,
            wait_minutes,
            mode,
            mock,
            mock_pr_numbers_json,
        } => {
            let layer = parse_layer(&layer)?;
            let mode = workflow::WaitMode::from_str(&mode)?;
            let mock_pr_numbers_json = match mock_pr_numbers_json {
                Some(json_str) => {
                    let parsed: Vec<u64> = serde_json::from_str(&json_str).map_err(|e| {
                        AppError::Validation(format!("Invalid mock-pr-numbers-json: {}", e))
                    })?;
                    Some(parsed)
                }
                None => None,
            };
            let options = workflow::WorkflowWaitPrsOptions {
                layer,
                base_branch,
                run_started_at,
                wait_minutes,
                mode,
                mock,
                mock_pr_numbers_json,
            };
            let output = workflow::wait_prs(options)?;
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
        WorkflowCleanupCommands::ProcessedIssue { issue_file, commit, push } => {
            let options =
                workflow::WorkflowCleanupProcessedIssueOptions { issue_file, commit, push };
            let output = workflow::cleanup_processed_issue(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_pr(command: WorkflowPrCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow;

    match command {
        WorkflowPrCommands::LabelFromBranch { branch } => {
            let options = workflow::WorkflowPrLabelOptions { branch };
            let output = workflow::pr_label_from_branch(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

fn run_workflow_matrix(command: WorkflowMatrixCommands) -> Result<(), AppError> {
    use crate::app::commands::workflow::{self, matrix};

    match command {
        WorkflowMatrixCommands::Workstreams => {
            let options = matrix::MatrixWorkstreamsOptions {};
            let output = matrix::workstreams(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowMatrixCommands::Roles { layer, workstreams_json } => {
            let layer = parse_layer(&layer)?;
            let workstreams_json: matrix::RolesWorkstreamsInput =
                serde_json::from_str(&workstreams_json).map_err(|e| {
                    AppError::Validation(format!("Invalid workstreams-json: {}", e))
                })?;
            let options = matrix::MatrixRolesOptions { layer, workstreams_json };
            let output = matrix::roles(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowMatrixCommands::PendingWorkstreams { workstreams_json, mock } => {
            let workstreams_json: matrix::PendingWorkstreamsInput =
                serde_json::from_str(&workstreams_json).map_err(|e| {
                    AppError::Validation(format!("Invalid workstreams-json: {}", e))
                })?;
            let options = matrix::MatrixPendingWorkstreamsOptions { workstreams_json, mock };
            let output = matrix::pending_workstreams(options)?;
            workflow::write_workflow_output(&output)
        }
        WorkflowMatrixCommands::Routing { workstreams_json, routing_labels } => {
            let workstreams_json: matrix::RoutingWorkstreamsInput =
                serde_json::from_str(&workstreams_json).map_err(|e| {
                    AppError::Validation(format!("Invalid workstreams-json: {}", e))
                })?;
            let options = matrix::MatrixRoutingOptions { workstreams_json, routing_labels };
            let output = matrix::routing(options)?;
            workflow::write_workflow_output(&output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
