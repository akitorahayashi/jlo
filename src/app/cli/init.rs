//! Init command implementation.

use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum InitCommands {
    /// Create .jules/ workspace structure
    #[clap(visible_alias = "s")]
    Scaffold,
    /// Install GitHub Actions workflow kit
    #[clap(visible_alias = "w")]
    Workflows {
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
}

pub fn run_init(command: Option<InitCommands>) -> Result<(), AppError> {
    match command.unwrap_or(InitCommands::Scaffold) {
        InitCommands::Scaffold => {
            crate::api::init()?;
            println!("✅ Initialized .jules/ workspace");
            Ok(())
        }
        InitCommands::Workflows { remote, self_hosted } => {
            let mode = if remote {
                crate::domain::WorkflowRunnerMode::Remote
            } else if self_hosted {
                crate::domain::WorkflowRunnerMode::SelfHosted
            } else {
                return Err(AppError::MissingArgument(
                    "Runner mode is required. Use --remote or --self-hosted.".into(),
                ));
            };
            crate::api::init_workflows(mode)?;
            println!("✅ Installed workflow kit ({})", mode.label());
            Ok(())
        }
    }
}
