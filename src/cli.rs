use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::WitnessError;

#[derive(Parser, Debug)]
#[command(name = "witness", version, about = "Reproducible command evidence recorder")]
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
    pub fn resolve_repo(&self) -> Result<PathBuf, WitnessError> {
        if let Some(ref repo) = self.repo {
            return Ok(repo.clone());
        }
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }
        std::env::current_dir().map_err(WitnessError::Io)
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
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
}
