use std::path::PathBuf;

use clap::{Parser, Subcommand};
use jlo::AppError;

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
    Init,
    /// Create a new role from a layer template
    #[clap(visible_alias = "tp")]
    Template {
        /// Layer: observers, deciders, planners, or implementers
        #[arg(short, long)]
        layer: Option<String>,
        /// Name for the new role
        #[arg(short, long)]
        name: Option<String>,
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
enum RunLayer {
    /// Run observer agents
    Observers {
        /// Specific roles to run (default: all from config)
        #[arg(long)]
        role: Option<Vec<String>>,
        /// Show assembled prompts without executing
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
    },
    /// Run decider agents
    Deciders {
        /// Specific roles to run (default: all from config)
        #[arg(long)]
        role: Option<Vec<String>>,
        /// Show assembled prompts without executing
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
    },
    /// Run planner agents
    Planners {
        /// Specific roles to run (default: all from config)
        #[arg(long)]
        role: Option<Vec<String>>,
        /// Show assembled prompts without executing
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
    },
    /// Run implementer agents
    Implementers {
        /// Specific roles to run (default: all from config)
        #[arg(long)]
        role: Option<Vec<String>>,
        /// Show assembled prompts without executing
        #[arg(long)]
        dry_run: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Local issue file path (required for implementers)
        #[arg(long)]
        issue: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), AppError> = match cli.command {
        Commands::Init => jlo::init(),
        Commands::Template { layer, name } => {
            jlo::template(layer.as_deref(), name.as_deref()).map(|_| ())
        }
        Commands::Setup { command } => match command {
            SetupCommands::Gen { path } => run_setup_gen(path),
            SetupCommands::List { detail } => run_setup_list(detail),
        },
        Commands::Run { layer } => run_agents(layer),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_agents(layer: RunLayer) -> Result<(), AppError> {
    use jlo::domain::Layer;

    let (target_layer, roles, dry_run, branch, issue) = match layer {
        RunLayer::Observers { role, dry_run, branch } => {
            (Layer::Observers, role, dry_run, branch, None)
        }
        RunLayer::Deciders { role, dry_run, branch } => {
            (Layer::Deciders, role, dry_run, branch, None)
        }
        RunLayer::Planners { role, dry_run, branch } => {
            (Layer::Planners, role, dry_run, branch, None)
        }
        RunLayer::Implementers { role, dry_run, branch, issue } => {
            (Layer::Implementers, role, dry_run, branch, issue)
        }
    };

    let result = jlo::run(target_layer, roles, dry_run, branch, issue)?;

    if !result.dry_run && !result.roles.is_empty() && !result.sessions.is_empty() {
        println!("✅ Created {} Jules session(s)", result.sessions.len());
    }

    Ok(())
}

fn run_setup_gen(path: Option<PathBuf>) -> Result<(), AppError> {
    let components = jlo::setup_gen(path.as_deref())?;
    println!("✅ Generated install.sh with {} component(s)", components.len());
    for (i, name) in components.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    Ok(())
}

fn run_setup_list(detail: Option<String>) -> Result<(), AppError> {
    if let Some(component) = detail {
        let info = jlo::setup_detail(&component)?;
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
        let components = jlo::setup_list()?;
        println!("Available components:");
        for comp in components {
            println!("  {} - {}", comp.name, comp.summary);
        }
    }
    Ok(())
}
