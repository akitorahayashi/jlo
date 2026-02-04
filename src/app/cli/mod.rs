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
        dry_run: bool,
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
    /// Export scheduling matrices
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommands,
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
        /// Overwrite existing kit-owned files
        #[arg(short = 'o', long)]
        overwrite: bool,
    },
}

#[derive(Subcommand)]
enum RunLayer {
    /// Run narrator agent (summarizes codebase changes)
    #[clap(visible_alias = "n")]
    Narrators {
        /// Show assembled prompts without executing
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
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
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
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
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
    },
    /// Run planner agent (single-role, issue-driven)
    #[clap(visible_alias = "p")]
    Planners {
        /// Local issue file path (required)
        issue: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
    },
    /// Run implementer agent (single-role, issue-driven)
    #[clap(visible_alias = "i")]
    Implementers {
        /// Local issue file path (required)
        issue: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
    },
}

#[derive(Subcommand)]
enum ScheduleCommands {
    /// Export schedule data for automation
    Export {
        /// Scope: workstreams or roles
        #[arg(long)]
        scope: String,
        /// Layer (required for roles scope)
        #[arg(long)]
        layer: Option<String>,
        /// Workstream (required for roles scope)
        #[arg(long)]
        workstream: Option<String>,
        /// Output format (default: github-matrix)
        #[arg(long, default_value = "github-matrix")]
        format: String,
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
        Commands::Update { dry_run, adopt_managed } => {
            run_update(dry_run, adopt_managed).map(|_| 0)
        }
        Commands::Template { layer, name, workstream } => {
            crate::app::api::template(layer.as_deref(), name.as_deref(), workstream.as_deref())
                .map(|_| 0)
        }
        Commands::Setup { command } => match command {
            SetupCommands::Gen { path } => run_setup_gen(path).map(|_| 0),
            SetupCommands::List { detail } => run_setup_list(detail).map(|_| 0),
        },
        Commands::Run { layer } => run_agents(layer).map(|_| 0),
        Commands::Schedule { command } => run_schedule(command).map(|_| 0),
        Commands::Workstreams { command } => run_workstreams(command).map(|_| 0),
        Commands::Doctor { fix, strict, workstream } => run_doctor(fix, strict, workstream),
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
        InitCommands::Scaffold => crate::app::api::init(),
        InitCommands::Workflows { remote, self_hosted, overwrite } => {
            let mode = if remote {
                crate::domain::WorkflowRunnerMode::Remote
            } else if self_hosted {
                crate::domain::WorkflowRunnerMode::SelfHosted
            } else {
                return Err(AppError::config_error(
                    "Runner mode is required. Use --remote or --self-hosted.",
                ));
            };
            crate::app::api::init_workflows(mode, overwrite)
        }
    }
}

fn run_update(dry_run: bool, adopt_managed: bool) -> Result<(), AppError> {
    let result = crate::app::api::update(dry_run, adopt_managed)?;

    if !result.dry_run {
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

    let (target_layer, roles, workstream, scheduled, dry_run, branch, issue) = match layer {
        RunLayer::Narrators { dry_run, branch } => {
            (Layer::Narrators, None, None, false, dry_run, branch, None)
        }
        RunLayer::Observers { role, dry_run, branch, workstream, scheduled } => {
            (Layer::Observers, role, workstream, scheduled, dry_run, branch, None)
        }
        RunLayer::Deciders { role, dry_run, branch, workstream, scheduled } => {
            (Layer::Deciders, role, workstream, scheduled, dry_run, branch, None)
        }
        RunLayer::Planners { dry_run, branch, issue } => {
            (Layer::Planners, None, None, false, dry_run, branch, Some(issue))
        }
        RunLayer::Implementers { dry_run, branch, issue } => {
            (Layer::Implementers, None, None, false, dry_run, branch, Some(issue))
        }
    };

    let result =
        crate::app::api::run(target_layer, roles, workstream, scheduled, dry_run, branch, issue)?;

    if !result.dry_run && !result.roles.is_empty() && !result.sessions.is_empty() {
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

fn run_schedule(command: ScheduleCommands) -> Result<(), AppError> {
    match command {
        ScheduleCommands::Export { scope, layer, workstream, format } => {
            let scope = parse_schedule_scope(&scope)?;
            let format = parse_schedule_format(&format)?;
            let layer = match layer {
                Some(value) => Some(parse_layer(&value)?),
                None => None,
            };

            let output = crate::app::api::schedule_export(crate::ScheduleExportOptions {
                scope,
                layer,
                workstream,
                format,
            })?;

            print_json(&output)
        }
    }
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

fn parse_schedule_scope(scope: &str) -> Result<crate::ScheduleExportScope, AppError> {
    match scope {
        "workstreams" => Ok(crate::ScheduleExportScope::Workstreams),
        "roles" => Ok(crate::ScheduleExportScope::Roles),
        _ => Err(AppError::config_error("Invalid schedule scope")),
    }
}

fn parse_schedule_format(format: &str) -> Result<crate::ScheduleExportFormat, AppError> {
    match format {
        "github-matrix" => Ok(crate::ScheduleExportFormat::GithubMatrix),
        _ => Err(AppError::config_error("Invalid schedule format")),
    }
}

fn parse_workstream_format(format: &str) -> Result<crate::WorkstreamInspectFormat, AppError> {
    match format {
        "json" => Ok(crate::WorkstreamInspectFormat::Json),
        "yaml" => Ok(crate::WorkstreamInspectFormat::Yaml),
        _ => Err(AppError::config_error("Invalid workstream inspect format")),
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|err| AppError::config_error(format!("Failed to serialize output: {}", err)))?;
    println!("{}", json);
    Ok(())
}

fn print_yaml<T: serde::Serialize>(value: &T) -> Result<(), AppError> {
    let yaml = serde_yaml::to_string(value)
        .map_err(|err| AppError::config_error(format!("Failed to serialize output: {}", err)))?;
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
