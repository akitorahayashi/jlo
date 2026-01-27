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
    /// Create .jules/ workspace structure with 4-layer architecture
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
        /// Layer: observers, deciders, planners, or implementers
        #[arg(short, long)]
        layer: Option<String>,
        /// Name for the new role
        #[arg(short, long)]
        name: Option<String>,
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
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
