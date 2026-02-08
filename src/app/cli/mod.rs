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
    /// Initialize .jlo/ control plane and install workflow kit
    #[clap(visible_alias = "i")]
    Init {
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
    /// Advance .jlo/ control-plane version pin
    #[clap(visible_alias = "u")]
    Update {
        /// Show planned changes without applying
        #[arg(long)]
        prompt_preview: bool,
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
    /// Create a new role or workstream under .jlo/
    #[clap(visible_alias = "c")]
    Create {
        #[command(subcommand)]
        command: CreateCommands,
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
        /// Limit checks to a specific workstream
        #[arg(long)]
        workstream: Option<String>,
    },
    /// Remove jlo-managed assets (branch + workflows)
    Deinit,
}

#[derive(Subcommand)]
enum CreateCommands {
    /// Create a new workstream under .jlo/workstreams/
    Workstream {
        /// Name for the new workstream
        name: String,
    },
    /// Create a new role under .jlo/roles/<layer>/roles/
    Role {
        /// Layer (observers, deciders, innovators)
        layer: String,
        /// Name for the new role
        name: String,
    },
}

/// Entry point for the CLI.
pub fn run() {
    let cli = Cli::parse();

    let result: Result<i32, AppError> = match cli.command {
        Commands::Init { remote, self_hosted } => init::run_init(remote, self_hosted).map(|_| 0),
        Commands::Update { prompt_preview } => run_update(prompt_preview).map(|_| 0),
        Commands::Template { layer, name, workstream } => {
            run_template(layer, name, workstream).map(|_| 0)
        }
        Commands::Create { command } => run_create(command).map(|_| 0),
        Commands::Setup { command } => match command {
            setup::SetupCommands::Gen { path } => setup::run_setup_gen(path).map(|_| 0),
            setup::SetupCommands::List { detail } => setup::run_setup_list(detail).map(|_| 0),
        },
        Commands::Run { layer } => run::run_agents(layer).map(|_| 0),
        Commands::Workflow { command } => workflow::run_workflow(command).map(|_| 0),
        Commands::Doctor { strict, workstream } => doctor::run_doctor(strict, workstream),
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

fn run_update(prompt_preview: bool) -> Result<(), AppError> {
    let result = crate::app::api::update(prompt_preview)?;

    if !result.prompt_preview {
        if result.created.is_empty() && result.previous_version == env!("CARGO_PKG_VERSION") {
            println!("✅ Workspace already up to date");
        } else {
            println!("✅ Updated workspace to version {}", env!("CARGO_PKG_VERSION"));
            if !result.created.is_empty() {
                println!("  Created {} file(s)", result.created.len());
            }
        }
    }

    Ok(())
}

fn run_create(command: CreateCommands) -> Result<(), AppError> {
    let outcome = match command {
        CreateCommands::Workstream { name } => crate::app::api::create_workstream(&name)?,
        CreateCommands::Role { layer, name } => crate::app::api::create_role(&layer, &name)?,
    };

    let entity_type = match &outcome {
        crate::app::api::CreateOutcome::Role { .. } => "role",
        crate::app::api::CreateOutcome::Workstream { .. } => "workstream",
    };
    println!("✅ Created new {} at {}/", entity_type, outcome.display_path());
    Ok(())
}
