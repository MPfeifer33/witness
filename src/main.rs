mod capture;
mod cli;
mod report;
mod store;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = run(&cli);
    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            if cli.is_json() {
                let err_json = serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": e.error_code(),
                        "message": e.to_string(),
                    }
                });
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&err_json).unwrap_or_else(|_| format!(
                        "{{\"ok\":false,\"error\":{{\"message\":\"{e}\"}}}}"
                    ))
                );
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(code);
        }
    }
}

fn run(cli: &Cli) -> Result<(), WitnessError> {
    match &cli.command {
        Command::Run { command, tag } => {
            let repo = cli.resolve_repo()?;
            let evidence = capture::run_and_capture(&repo, command, tag.as_deref())?;
            let id = store::save(&repo, &evidence)?;

            if cli.is_json() {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "evidence_id": id,
                        "exit_code": evidence.exit_code,
                        "duration_ms": evidence.duration_ms,
                        "passed": evidence.exit_code == 0,
                    }))?
                );
            } else {
                let icon = if evidence.exit_code == 0 {
                    "✓"
                } else {
                    "✗"
                };
                println!(
                    "{icon} Command completed (exit {}), evidence saved: {id}",
                    evidence.exit_code
                );
                println!("  Duration: {}ms", evidence.duration_ms);
            }
            Ok(())
        }
        Command::List { limit } => {
            let repo = cli.resolve_repo()?;
            let list = store::list(&repo, *limit)?;
            report::print_list(&list, cli.is_json())?;
            Ok(())
        }
        Command::Show { id } => {
            let repo = cli.resolve_repo()?;
            let evidence = store::load(&repo, id)?;
            report::print_evidence(&evidence, cli.is_json())?;
            Ok(())
        }
        Command::Verify { id } => {
            let repo = cli.resolve_repo()?;
            let evidence = store::load(&repo, id)?;
            let verification = store::verify(&repo, &evidence)?;

            if cli.is_json() {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "evidence_id": id,
                        "verified": verification.verified,
                        "reason": verification.reason.as_str(),
                    }))?
                );
            } else if verification.verified {
                println!("✓ Evidence {id} verified — bundle hash matches");
            } else {
                println!(
                    "✗ Evidence {id} FAILED verification — {}",
                    verification.reason.human_message()
                );
            }
            Ok(())
        }
        Command::Doctor { limit, strict } => {
            let repo = cli.resolve_repo()?;
            let witness_dir = store::witness_dir(&repo);
            let evidence_dir = store::evidence_dir(&repo);
            let paths_usable = repo.exists()
                && repo.is_dir()
                && (!witness_dir.exists() || witness_dir.is_dir())
                && (!evidence_dir.exists() || evidence_dir.is_dir());
            let list = if paths_usable {
                store::list(&repo, *limit)?
            } else {
                store::EvidenceList::default()
            };
            let doctor =
                report::print_doctor(&repo, &witness_dir, &evidence_dir, &list, cli.is_json())?;
            if *strict {
                std::process::exit(doctor.action_level.strict_exit_code());
            }
            Ok(())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WitnessError {
    #[error("{0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl WitnessError {
    pub fn exit_code(&self) -> i32 {
        match self {
            WitnessError::Validation(_) => 1,
            WitnessError::NotFound(_) => 3,
            WitnessError::Io(_) => 2,
            WitnessError::Json(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            WitnessError::Validation(_) => "validation_error",
            WitnessError::NotFound(_) => "not_found",
            WitnessError::Io(_) => "io_error",
            WitnessError::Json(_) => "json_error",
        }
    }
}
