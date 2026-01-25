use clap::{Parser, Subcommand};
use rs_cli_tmpl::error::AppError;
use rs_cli_tmpl::{add, delete, list};

#[derive(Parser)]
#[command(name = "rs-cli-tmpl")]
#[command(version)]
#[command(
    about = "Reference architecture for building Rust CLI tools",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(visible_alias = "a")]
    Add {
        id: String,
        #[clap(short, long)]
        content: String,
    },
    #[clap(visible_alias = "ls")]
    List,
    #[clap(visible_alias = "rm")]
    Delete { id: String },
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), AppError> = match cli.command {
        Commands::Add { id, content } => add(&id, &content),
        Commands::List => list().map(|_| ()),
        Commands::Delete { id } => delete(&id),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
