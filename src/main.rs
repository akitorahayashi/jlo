use clap::{Parser, Subcommand};
use jo::error::AppError;

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
    /// Create .jules/ minimal structure
    #[clap(visible_alias = "i")]
    Init {
        /// Force initialization even if workspace exists
        #[clap(short, long)]
        force: bool,
    },
    /// Update jo-managed files (README, version)
    #[clap(visible_alias = "u")]
    Update,
    /// Interactive role selection and scheduler prompt generation
    #[clap(visible_alias = "r")]
    Role,
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), AppError> = match cli.command {
        Commands::Init { force } => jo::init(force),
        Commands::Update => jo::update(),
        Commands::Role => jo::role_interactive().map(|_| ()),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
