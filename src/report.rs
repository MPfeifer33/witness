use crate::capture::Evidence;
use crate::store::{EvidenceEntry, EvidenceList};
use crate::WitnessError;
use std::path::Path;

const DOCTOR_SCHEMA_VERSION: &str = "witness.doctor.v1";

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentDoctor {
    pub schema_version: String,
    pub status: DoctorStatus,
    pub action_level: ActionLevel,
    pub gates: AgentGates,
    pub store: EvidenceStoreStatus,
    pub evidence_count: usize,
    pub listed_evidence_count: usize,
    pub invalid_count: usize,
    pub latest_evidence: Option<EvidenceEntry>,
    pub invalid: Vec<crate::store::InvalidEvidenceEntry>,
    pub advice: String,
    pub recommendations: Vec<String>,
    pub recommended_commands: Vec<RecommendedCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Ready,
    Blocked,
}

impl DoctorStatus {
    fn label(self) -> &'static str {
        match self {
            DoctorStatus::Ready => "ready",
            DoctorStatus::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionLevel {
    None,
    Review,
    Stop,
}

impl ActionLevel {
    fn label(self) -> &'static str {
        match self {
            ActionLevel::None => "none",
            ActionLevel::Review => "review",
            ActionLevel::Stop => "stop",
        }
    }

    pub fn strict_exit_code(self) -> i32 {
        match self {
            ActionLevel::None => 0,
            ActionLevel::Review | ActionLevel::Stop => 30,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentGates {
    pub repo_exists: bool,
    pub repo_is_directory: bool,
    pub witness_path_usable: bool,
    pub evidence_path_usable: bool,
    pub invalid_bundles_clear: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvidenceStoreStatus {
    pub repo_path: String,
    pub witness_dir: String,
    pub evidence_dir: String,
    pub witness_dir_exists: bool,
    pub evidence_dir_exists: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecommendedCommand {
    pub kind: RecommendationKind,
    pub command: Option<String>,
    pub argv: Option<Vec<String>>,
    pub label: String,
    pub reason: String,
    pub reason_code: String,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationKind {
    Command,
    Manual,
}

pub fn print_list(list: &EvidenceList, is_json: bool) -> Result<(), WitnessError> {
    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "evidence": &list.entries,
                "invalid_count": list.invalid_count(),
                "invalid": &list.invalid,
            }))?
        );
    } else {
        if list.entries.is_empty() && list.invalid.is_empty() {
            println!("No evidence recorded yet.");
        } else {
            if !list.entries.is_empty() {
                println!("witness: {} evidence bundle(s)", list.entries.len());
                println!();
                for e in &list.entries {
                    let icon = if e.exit_code == 0 { "✓" } else { "✗" };
                    let tag_str = e
                        .tag
                        .as_deref()
                        .map(|t| format!(" [{t}]"))
                        .unwrap_or_default();
                    println!(
                        "  {icon} {} `{}`{} ({}ms, exit {})",
                        e.id,
                        truncate(&e.command, 50),
                        tag_str,
                        e.duration_ms,
                        e.exit_code,
                    );
                }
            }

            if !list.invalid.is_empty() {
                if !list.entries.is_empty() {
                    println!();
                }
                println!("Invalid evidence bundle(s): {}", list.invalid_count());
                for invalid in &list.invalid {
                    println!("  ! {} ({})", invalid.path, invalid.reason);
                }
            }
        }
    }
    Ok(())
}

pub fn print_doctor(
    repo: &Path,
    witness_dir: &Path,
    evidence_dir: &Path,
    list: &EvidenceList,
    is_json: bool,
) -> Result<AgentDoctor, WitnessError> {
    let doctor = build_doctor(repo, witness_dir, evidence_dir, list);

    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "schema_version": &doctor.schema_version,
                "status": doctor.status,
                "action_level": doctor.action_level,
                "doctor": &doctor,
                "gates": &doctor.gates,
                "store": &doctor.store,
                "evidence_count": doctor.evidence_count,
                "listed_evidence_count": doctor.listed_evidence_count,
                "invalid_count": doctor.invalid_count,
                "latest_evidence": &doctor.latest_evidence,
                "invalid": &doctor.invalid,
                "advice": &doctor.advice,
                "recommendations": &doctor.recommendations,
                "recommended_commands": &doctor.recommended_commands,
            }))?
        );
    } else {
        println!("witness doctor:");
        println!();
        println!(
            "  Status: {} ({})",
            doctor.status.label(),
            doctor.action_level.label()
        );
        println!("  Repo: {}", doctor.store.repo_path);
        println!("  Store: {}", doctor.store.witness_dir);
        println!("  Evidence dir: {}", doctor.store.evidence_dir);
        println!("  Evidence bundles: {}", doctor.evidence_count);
        println!("  Invalid bundles: {}", doctor.invalid_count);
        println!();
        println!("  Gates:");
        println!("    repo exists: {}", doctor.gates.repo_exists);
        println!("    repo is directory: {}", doctor.gates.repo_is_directory);
        println!(
            "    witness path usable: {}",
            doctor.gates.witness_path_usable
        );
        println!(
            "    evidence path usable: {}",
            doctor.gates.evidence_path_usable
        );
        println!(
            "    invalid bundles clear: {}",
            doctor.gates.invalid_bundles_clear
        );

        if let Some(latest) = &doctor.latest_evidence {
            println!();
            println!("  Latest evidence:");
            println!(
                "    {} `{}` (exit {}, {}ms)",
                latest.id,
                truncate(&latest.command, 60),
                latest.exit_code,
                latest.duration_ms
            );
        }

        if !doctor.invalid.is_empty() {
            println!();
            println!("  Invalid evidence bundle(s):");
            for invalid in &doctor.invalid {
                println!("    ! {} ({})", invalid.path, invalid.reason);
            }
        }

        println!();
        println!("  Advice: {}", doctor.advice);
        println!();
        println!("  Recommended next steps:");
        for recommendation in &doctor.recommended_commands {
            println!("    - {}", recommendation.label);
        }
    }

    Ok(doctor)
}

pub fn print_evidence(evidence: &Evidence, is_json: bool) -> Result<(), WitnessError> {
    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "evidence": evidence,
            }))?
        );
    } else {
        let icon = if evidence.exit_code == 0 {
            "✓"
        } else {
            "✗"
        };
        println!("witness evidence: {icon} {}", evidence.id);
        println!();
        println!("  Command: {}", evidence.command);
        println!("  Timestamp: {}", evidence.timestamp);
        println!("  Duration: {}ms", evidence.duration_ms);
        println!("  Exit code: {}", evidence.exit_code);

        if let Some(ref tag) = evidence.tag {
            println!("  Tag: {tag}");
        }

        println!("  CWD: {}", evidence.cwd);
        println!("  OS: {}", evidence.environment.os);
        println!("  User: {}", evidence.environment.user);

        if let Some(ref v) = evidence.environment.rust_version {
            println!("  Rust: {v}");
        }
        if let Some(ref v) = evidence.environment.node_version {
            println!("  Node: {v}");
        }

        if let Some(ref git) = evidence.git_context {
            println!(
                "  Git: {} @ {}{}",
                git.branch,
                git.head_sha,
                if git.dirty { " (dirty)" } else { "" }
            );
        }

        println!("  Bundle hash: {}", hash_preview(&evidence.bundle_hash));

        if !evidence.stdout.is_empty() {
            println!();
            println!("  --- stdout ---");
            for line in evidence.stdout.lines().take(20) {
                println!("  {line}");
            }
            if evidence.stdout.lines().count() > 20 {
                println!(
                    "  ... ({} more lines)",
                    evidence.stdout.lines().count() - 20
                );
            }
        }

        if !evidence.stderr.is_empty() {
            println!();
            println!("  --- stderr ---");
            for line in evidence.stderr.lines().take(10) {
                println!("  {line}");
            }
            if evidence.stderr.lines().count() > 10 {
                println!(
                    "  ... ({} more lines)",
                    evidence.stderr.lines().count() - 10
                );
            }
        }
    }
    Ok(())
}

fn build_doctor(
    repo: &Path,
    witness_dir: &Path,
    evidence_dir: &Path,
    list: &EvidenceList,
) -> AgentDoctor {
    let gates = AgentGates {
        repo_exists: repo.exists(),
        repo_is_directory: repo.is_dir(),
        witness_path_usable: !witness_dir.exists() || witness_dir.is_dir(),
        evidence_path_usable: !evidence_dir.exists() || evidence_dir.is_dir(),
        invalid_bundles_clear: list.invalid.is_empty(),
    };
    let action_level = action_level_for(&gates);
    let status = status_for(action_level);
    let recommended_commands = recommended_commands_for(action_level, &gates, list);
    let recommendations = recommended_commands
        .iter()
        .map(recommendation_label)
        .collect();

    AgentDoctor {
        schema_version: DOCTOR_SCHEMA_VERSION.to_string(),
        status,
        action_level,
        gates,
        store: EvidenceStoreStatus {
            repo_path: repo.display().to_string(),
            witness_dir: witness_dir.display().to_string(),
            evidence_dir: evidence_dir.display().to_string(),
            witness_dir_exists: witness_dir.exists(),
            evidence_dir_exists: evidence_dir.exists(),
        },
        evidence_count: list.total_valid_count,
        listed_evidence_count: list.entries.len(),
        invalid_count: list.invalid_count(),
        latest_evidence: list.entries.first().cloned(),
        invalid: list.invalid.clone(),
        advice: advice_for(action_level, list),
        recommendations,
        recommended_commands,
    }
}

fn action_level_for(gates: &AgentGates) -> ActionLevel {
    if !gates.repo_exists
        || !gates.repo_is_directory
        || !gates.witness_path_usable
        || !gates.evidence_path_usable
    {
        ActionLevel::Stop
    } else if !gates.invalid_bundles_clear {
        ActionLevel::Review
    } else {
        ActionLevel::None
    }
}

fn status_for(action_level: ActionLevel) -> DoctorStatus {
    match action_level {
        ActionLevel::None => DoctorStatus::Ready,
        ActionLevel::Review | ActionLevel::Stop => DoctorStatus::Blocked,
    }
}

fn advice_for(action_level: ActionLevel, list: &EvidenceList) -> String {
    match action_level {
        ActionLevel::None if list.total_valid_count == 0 => {
            "evidence store is usable and empty; record validation with witness run when you need receipts".to_string()
        }
        ActionLevel::None => {
            "evidence store is usable; verify evidence IDs before citing them as proof".to_string()
        }
        ActionLevel::Review => {
            "invalid local evidence bundles are present; do not cite those bundles until inspected or removed".to_string()
        }
        ActionLevel::Stop => {
            "evidence store path is unusable; fix the repo or .agent-witness path before recording receipts".to_string()
        }
    }
}

fn recommended_commands_for(
    action_level: ActionLevel,
    gates: &AgentGates,
    list: &EvidenceList,
) -> Vec<RecommendedCommand> {
    let mut commands = Vec::new();

    if matches!(action_level, ActionLevel::Stop) {
        if !gates.repo_exists || !gates.repo_is_directory {
            commands.push(manual_recommendation(
                "Use --repo with an existing project directory",
                "repo_unavailable",
                "the resolved repo path does not exist or is not a directory",
                true,
            ));
        }
        if !gates.witness_path_usable || !gates.evidence_path_usable {
            commands.push(manual_recommendation(
                "Remove or rename the file blocking .agent-witness storage",
                "evidence_path_blocked",
                ".agent-witness or .agent-witness/evidence exists but is not a directory",
                true,
            ));
        }
        return commands;
    }

    if !gates.invalid_bundles_clear {
        commands.push(command_recommendation(
            "witness list",
            &["witness", "list"],
            "inspect_invalid_evidence",
            "list surfaces invalid local bundles with paths and parse/read reasons",
            true,
        ));
        commands.push(manual_recommendation(
            "Do not cite invalid bundles as evidence until reviewed",
            "invalid_evidence_review",
            "invalid evidence cannot be trusted as a receipt",
            true,
        ));
    }

    if commands.is_empty() {
        if let Some(latest) = list.entries.first() {
            commands.push(command_recommendation(
                format!("witness verify {}", latest.id).as_str(),
                &["witness", "verify", &latest.id],
                "verify_latest_evidence",
                "verify the latest evidence bundle before citing it",
                false,
            ));
        } else {
            commands.push(command_recommendation(
                "witness run -- <command>",
                &["witness", "run", "--", "<command>"],
                "record_validation_evidence",
                "no evidence is recorded yet",
                false,
            ));
        }
    }

    commands
}

fn command_recommendation(
    label: &str,
    argv: &[&str],
    reason_code: &str,
    reason: &str,
    required: bool,
) -> RecommendedCommand {
    RecommendedCommand {
        kind: RecommendationKind::Command,
        command: Some(label.to_string()),
        argv: Some(argv.iter().map(|arg| (*arg).to_string()).collect()),
        label: label.to_string(),
        reason: reason.to_string(),
        reason_code: reason_code.to_string(),
        required,
    }
}

fn manual_recommendation(
    label: &str,
    reason_code: &str,
    reason: &str,
    required: bool,
) -> RecommendedCommand {
    RecommendedCommand {
        kind: RecommendationKind::Manual,
        command: None,
        argv: None,
        label: label.to_string(),
        reason: reason.to_string(),
        reason_code: reason_code.to_string(),
        required,
    }
}

fn recommendation_label(command: &RecommendedCommand) -> String {
    if command.required {
        format!("required: {} ({})", command.label, command.reason_code)
    } else {
        format!("optional: {} ({})", command.label, command.reason_code)
    }
}

fn truncate(s: &str, max: usize) -> String {
    let mut chars = s.chars();
    let preview: String = chars.by_ref().take(max).collect();
    if chars.next().is_none() {
        s.to_string()
    } else {
        format!("{preview}...")
    }
}

fn hash_preview(hash: &str) -> String {
    if hash.is_empty() {
        "<missing>".to_string()
    } else {
        hash.chars().take(16).collect()
    }
}
