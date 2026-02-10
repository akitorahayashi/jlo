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
    /// Initialize .jlo/ control plane and install workflow scaffold
    #[clap(visible_alias = "i")]
    Init {
        /// Install the GitHub-hosted runner workflow scaffold
        #[arg(
            short = 'r',
            long,
            conflicts_with = "self_hosted",
            required_unless_present = "self_hosted"
        )]
        remote: bool,
        /// Install the self-hosted runner workflow scaffold
        #[arg(short = 's', long, conflicts_with = "remote", required_unless_present = "remote")]
        self_hosted: bool,
    },
    /// Advance .jlo/ control-plane version pin
    #[clap(visible_alias = "u")]
    Update {
        /// Show planned changes without applying
        #[arg(long)]
        prompt_preview: bool,
    },
    /// Create a new role under .jlo/
    #[clap(visible_alias = "c")]
    Create {
        /// Layer (observers, deciders, innovators)
        layer: String,
        /// Name for the new role
        name: String,
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
        /// Treat warnings as failures
        #[arg(long)]
        strict: bool,
    },
    /// Remove jlo-managed assets (branch + workflows)
    Deinit,
}

/// Entry point for the CLI.
pub fn run() {
    let cli = Cli::parse();

    let result: Result<i32, AppError> = match cli.command {
        Commands::Init { remote, self_hosted } => init::run_init(remote, self_hosted).map(|_| 0),
        Commands::Update { prompt_preview } => run_update(prompt_preview).map(|_| 0),
        Commands::Create { layer, name } => run_create(layer, name).map(|_| 0),
        Commands::Setup { command } => match command {
            setup::SetupCommands::Gen { path } => setup::run_setup_gen(path).map(|_| 0),
            setup::SetupCommands::List { detail } => setup::run_setup_list(detail).map(|_| 0),
        },
        Commands::Run { layer } => run::run_agents(layer).map(|_| 0),
        Commands::Workflow { command } => workflow::run_workflow(command).map(|_| 0),
        Commands::Doctor { strict } => doctor::run_doctor(strict),
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

fn run_update(prompt_preview: bool) -> Result<(), AppError> {
    let result = crate::app::api::update(prompt_preview)?;

    if !result.prompt_preview {
        if !result.warnings.is_empty() {
            println!("⚠️  Update warnings:");
            for warning in &result.warnings {
                println!("  • {}", warning);
            }
        }

        if result.created.is_empty()
            && result.updated.is_empty()
            && !result.workflow_refreshed
            && result.previous_version == env!("CARGO_PKG_VERSION")
        {
            println!("✅ Workspace already up to date");
        } else {
            println!("✅ Updated workspace to version {}", env!("CARGO_PKG_VERSION"));
            if !result.created.is_empty() {
                println!("  Created {} file(s)", result.created.len());
            }
            if !result.updated.is_empty() {
                println!("  Refreshed {} managed default file(s)", result.updated.len());
            }
            if result.workflow_refreshed {
                println!("  Refreshed workflow scaffold");
            }
        }
    }

    Ok(())
}

fn run_create(layer: String, name: String) -> Result<(), AppError> {
    let outcome = crate::app::api::create_role(&layer, &name)?;

    println!("✅ Created new {} at {}/", outcome.entity_type(), outcome.display_path());
    Ok(())
}
