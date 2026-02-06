//! CLI Adapter.

mod deinit;
mod doctor;
mod init;
mod run;
mod setup;
mod workflow;

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
        command: Option<init::InitCommands>,
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
        command: setup::SetupCommands,
    },
    /// Execute Jules agents
    #[clap(visible_alias = "r")]
    Run {
        #[command(subcommand)]
        layer: run::RunLayer,
    },
    /// Workflow orchestration primitives for GitHub Actions
    #[clap(visible_alias = "wf")]
    Workflow {
        #[command(subcommand)]
        command: workflow::WorkflowCommands,
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

/// Entry point for the CLI.
pub fn run() {
    let cli = Cli::parse();

    let result: Result<i32, AppError> = match cli.command {
        Commands::Init { command } => init::run_init(command).map(|_| 0),
        Commands::Update { prompt_preview, adopt_managed } => {
            run_update(prompt_preview, adopt_managed).map(|_| 0)
        }
        Commands::Template { layer, name, workstream } => {
            run_template(layer, name, workstream).map(|_| 0)
        }
        Commands::Setup { command } => match command {
            setup::SetupCommands::Gen { path } => setup::run_setup_gen(path).map(|_| 0),
            setup::SetupCommands::List { detail } => setup::run_setup_list(detail).map(|_| 0),
        },
        Commands::Run { layer } => run::run_agents(layer).map(|_| 0),
        Commands::Workflow { command } => workflow::run_workflow(command).map(|_| 0),
        Commands::Doctor { fix, strict, workstream } => doctor::run_doctor(fix, strict, workstream),
        Commands::Deinit => deinit::run_deinit().map(|_| 0),
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
