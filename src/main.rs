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
    /// Read a role's prompt.yml and copy to clipboard
    #[clap(visible_alias = "a")]
    Assign {
        /// Role name or prefix (supports fuzzy matching)
        role: String,
        /// Optional paths to add to the prompt at execution time
        #[arg(trailing_var_arg = true)]
        paths: Vec<String>,
    },
    /// Create a new role from a layer template
    #[clap(visible_alias = "tp")]
    Template {
        /// Layer: observers, deciders, planners, or mergers
        #[arg(short, long)]
        layer: Option<String>,
        /// Name for the new role
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Delete old jules/* branches
    #[clap(visible_alias = "prn")]
    Prune {
        /// Delete branches older than N days
        #[arg(short, long)]
        days: u32,
        /// Preview branches without deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Setup compiler commands
    #[clap(visible_alias = "s")]
    Setup {
        #[command(subcommand)]
        command: SetupCommands,
    },
}

#[derive(Subcommand)]
enum SetupCommands {
    /// Initialize .jules/setup/ workspace
    Init {
        /// Directory to initialize (defaults to current directory)
        path: Option<PathBuf>,
    },
    /// Generate install.sh and env.toml
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

fn main() {
    let cli = Cli::parse();

    let result: Result<(), AppError> = match cli.command {
        Commands::Init => jlo::init(),
        Commands::Assign { role, paths } => jlo::assign(&role, &paths).map(|_| ()),
        Commands::Template { layer, name } => {
            jlo::template(layer.as_deref(), name.as_deref()).map(|_| ())
        }
        Commands::Prune { days, dry_run } => jlo::prune(days, dry_run),
        Commands::Setup { command } => match command {
            SetupCommands::Init { path } => run_setup_init(path),
            SetupCommands::Gen { path } => run_setup_gen(path),
            SetupCommands::List { detail } => run_setup_list(detail),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_setup_init(path: Option<PathBuf>) -> Result<(), AppError> {
    jlo::setup_init(path.as_deref())?;
    println!("✅ Initialized .jules/setup/ workspace");
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
