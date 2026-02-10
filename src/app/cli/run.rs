//! Run command implementation.

use std::path::PathBuf;

use crate::domain::AppError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum RunLayer {
    /// Run narrator agent (summarizes codebase changes)
    #[clap(visible_alias = "n")]
    Narrator {
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run observer agent (single role)
    #[clap(visible_alias = "o")]
    Observers {
        /// Role to run
        #[arg(short = 'r', long)]
        role: String,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run decider agent (single role)
    #[clap(visible_alias = "d")]
    Deciders {
        /// Role to run
        #[arg(short = 'r', long)]
        role: String,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run planner agent (single-role, issue-driven)
    #[clap(visible_alias = "p")]
    Planners {
        /// Local issue file path (required)
        issue: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run implementer agent (single-role, issue-driven)
    #[clap(visible_alias = "i")]
    Implementers {
        /// Local issue file path (required)
        issue: PathBuf,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
    /// Run innovator agent (single role)
    #[clap(visible_alias = "x")]
    Innovators {
        /// Role (persona) to run
        #[arg(short = 'r', long)]
        role: String,
        /// Execution phase (creation or refinement)
        #[arg(long)]
        phase: Option<String>,
        /// Show assembled prompts without executing
        #[arg(long, conflicts_with = "mock")]
        prompt_preview: bool,
        /// Override the starting branch
        #[arg(long)]
        branch: Option<String>,
        /// Run in mock mode (no Jules API, real git/GitHub operations)
        #[arg(long, conflicts_with = "prompt_preview")]
        mock: bool,
    },
}

pub fn run_agents(layer: RunLayer) -> Result<(), AppError> {
    use crate::domain::Layer;

    let (target_layer, role, prompt_preview, branch, issue, mock, phase) = match layer {
        RunLayer::Narrator { prompt_preview, branch, mock } => {
            (Layer::Narrators, None, prompt_preview, branch, None, mock, None)
        }
        RunLayer::Observers { role, prompt_preview, branch, mock } => {
            (Layer::Observers, Some(role), prompt_preview, branch, None, mock, None)
        }
        RunLayer::Deciders { role, prompt_preview, branch, mock } => {
            (Layer::Deciders, Some(role), prompt_preview, branch, None, mock, None)
        }
        RunLayer::Planners { prompt_preview, branch, issue, mock } => {
            (Layer::Planners, None, prompt_preview, branch, Some(issue), mock, None)
        }
        RunLayer::Implementers { prompt_preview, branch, issue, mock } => {
            (Layer::Implementers, None, prompt_preview, branch, Some(issue), mock, None)
        }
        RunLayer::Innovators { role, phase, prompt_preview, branch, mock } => {
            (Layer::Innovators, Some(role), prompt_preview, branch, None, mock, phase)
        }
    };

    let result =
        crate::app::api::run(target_layer, role, prompt_preview, branch, issue, mock, phase)?;

    if !result.prompt_preview && !result.roles.is_empty() && !result.sessions.is_empty() {
        println!("✅ Created {} Jules session(s)", result.sessions.len());
    }

    Ok(())
}
