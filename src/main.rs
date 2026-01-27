use clap::{Parser, Subcommand};
use jo::AppError;

#[derive(Parser)]
#[command(name = "jo")]
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
    /// Generate prompt for a role and copy to clipboard
    #[clap(visible_alias = "a")]
    Assign {
        /// Role name or prefix (supports fuzzy matching)
        role: String,
        /// Optional context paths to include in the prompt
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
        Commands::Init => jo::init(),
        Commands::Assign { role, paths } => jo::assign(&role, &paths).map(|_| ()),
        Commands::Template { layer, name } => {
            jo::template(layer.as_deref(), name.as_deref()).map(|_| ())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
