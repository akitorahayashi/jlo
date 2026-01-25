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
    /// Create .jules/ skeleton and source-of-truth docs
    #[clap(visible_alias = "i")]
    Init {
        /// Force initialization even if workspace exists
        #[clap(short, long)]
        force: bool,
    },
    /// Update jo-managed docs/templates under .jules/.jo/
    #[clap(visible_alias = "u")]
    Update {
        /// Force overwrite even if local modifications exist
        #[clap(short, long)]
        force: bool,
    },
    /// Print version info and detect local modifications
    #[clap(visible_alias = "st")]
    Status,
    /// Scaffold .jules/roles/<role_id>/ workspace
    #[clap(visible_alias = "r")]
    Role,
    /// Create a new session file under a role's sessions directory
    #[clap(visible_alias = "s")]
    Session {
        /// Role identifier
        role_id: String,
        /// Session slug for the filename
        #[clap(short, long)]
        slug: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), AppError> = match cli.command {
        Commands::Init { force } => jo::init(force),
        Commands::Update { force } => jo::update(force),
        Commands::Status => jo::status(),
        Commands::Role => jo::role_interactive().map(|_| ()),
        Commands::Session { role_id, slug } => jo::session(&role_id, slug.as_deref()).map(|_| ()),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
