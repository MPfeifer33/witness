use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};

use crate::capture::Evidence;
use crate::WitnessError;

const WITNESS_DIR: &str = ".agent-witness";
const EVIDENCE_DIR: &str = "evidence";

fn evidence_dir(repo: &Path) -> PathBuf {
    repo.join(WITNESS_DIR).join(EVIDENCE_DIR)
}

pub fn save(repo: &Path, evidence: &Evidence) -> Result<String, WitnessError> {
    let dir = evidence_dir(repo);
    std::fs::create_dir_all(&dir)?;

    // Write .gitignore
    let gitignore = repo.join(WITNESS_DIR).join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, "*\n")?;
    }

    let filename = format!("{}.json", evidence.id);
    let filepath = dir.join(&filename);
    let json = serde_json::to_string_pretty(evidence)?;
    std::fs::write(&filepath, json)?;

    Ok(evidence.id.clone())
}

pub fn list(repo: &Path, limit: usize) -> Result<Vec<EvidenceEntry>, WitnessError> {
    let dir = evidence_dir(repo);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<EvidenceEntry> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        .filter_map(|e| {
            let content = std::fs::read_to_string(e.path()).ok()?;
            let evidence: Evidence = serde_json::from_str(&content).ok()?;
            Some(EvidenceEntry {
                id: evidence.id,
                timestamp: evidence.timestamp,
                command: evidence.command,
                exit_code: evidence.exit_code,
                duration_ms: evidence.duration_ms,
                tag: evidence.tag,
            })
        })
        .collect();

    // Sort by timestamp descending
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries.truncate(limit);
    Ok(entries)
}

pub fn load(repo: &Path, id: &str) -> Result<Evidence, WitnessError> {
    let dir = evidence_dir(repo);
    let filepath = dir.join(format!("{id}.json"));

    if !filepath.exists() {
        return Err(WitnessError::NotFound(format!("Evidence {id} not found")));
    }

    let content = std::fs::read_to_string(&filepath)?;
    let evidence: Evidence = serde_json::from_str(&content)?;
    Ok(evidence)
}

pub fn verify(_repo: &Path, evidence: &Evidence) -> Result<bool, WitnessError> {
    // Recompute bundle hash and compare
    let mut hasher = Sha256::new();
    hasher.update(evidence.command.as_bytes());
    hasher.update(evidence.timestamp.as_bytes());
    hasher.update(evidence.exit_code.to_string().as_bytes());
    hasher.update(evidence.stdout.as_bytes());
    hasher.update(evidence.stderr.as_bytes());
    let hash = hasher.finalize();
    let computed = format!("{:x}", hash);

    Ok(computed == evidence.bundle_hash)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EvidenceEntry {
    pub id: String,
    pub timestamp: String,
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub tag: Option<String>,
}
