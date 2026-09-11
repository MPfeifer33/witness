pub use agent_tools_core::Format as OutputFormat;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::WitnessError;

#[derive(Parser, Debug)]
#[command(
    name = "witness",
    version,
    about = "Reproducible command evidence recorder"
)]
pub struct Cli {
    /// Project root override
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    /// `--repo` > `AGENT_REPO` > detected `.git` root > cwd (see agent-tools-core).
    pub fn resolve_repo(&self) -> Result<PathBuf, WitnessError> {
        Ok(agent_tools_core::resolve_repo(self.repo.as_deref())?)
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run a command and record evidence
    Run {
        /// Command to execute (use -- to pass flags)
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
        /// Tag for categorization
        #[arg(long)]
        tag: Option<String>,
        /// Always exit 0 after recording, even when the wrapped command failed
        /// (legacy behaviour; by default witness exits with the wrapped command's code)
        #[arg(long)]
        exit_zero: bool,
    },
    /// List recorded evidence
    List {
        /// Max entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Show details of a specific evidence bundle
    Show {
        /// Evidence ID
        id: String,
    },
    /// Verify evidence bundle integrity
    Verify {
        /// Evidence ID
        id: String,
    },
    /// Check evidence-store health for agents
    Doctor {
        /// Max evidence entries to include
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Exit non-zero according to action_level after printing the normal report
        #[arg(long)]
        strict: bool,
    },
}
