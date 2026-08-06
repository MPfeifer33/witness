use std::path::{Path, PathBuf};

use crate::capture::{compute_bundle_hash, Evidence};
use crate::WitnessError;

const WITNESS_DIR: &str = ".agent-witness";
const EVIDENCE_DIR: &str = "evidence";

pub fn witness_dir(repo: &Path) -> PathBuf {
    repo.join(WITNESS_DIR)
}

pub fn evidence_dir(repo: &Path) -> PathBuf {
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

pub fn list(repo: &Path, limit: usize) -> Result<EvidenceList, WitnessError> {
    let dir = evidence_dir(repo);
    if !dir.exists() {
        return Ok(EvidenceList::default());
    }

    let mut entries = Vec::new();
    let mut invalid = Vec::new();

    for entry in std::fs::read_dir(&dir)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                invalid.push(InvalidEvidenceEntry {
                    path: dir.display().to_string(),
                    reason: format!("read_dir error: {err}"),
                });
                continue;
            }
        };
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                invalid.push(InvalidEvidenceEntry {
                    path: display_evidence_path(repo, &path),
                    reason: format!("io error: {err}"),
                });
                continue;
            }
        };
        let evidence: Evidence = match serde_json::from_str(&content) {
            Ok(evidence) => evidence,
            Err(err) => {
                invalid.push(InvalidEvidenceEntry {
                    path: display_evidence_path(repo, &path),
                    reason: format!("json error: {err}"),
                });
                continue;
            }
        };
        entries.push(EvidenceEntry {
            id: evidence.id,
            timestamp: evidence.timestamp,
            command: evidence.command,
            exit_code: evidence.exit_code,
            duration_ms: evidence.duration_ms,
            tag: evidence.tag,
        });
    }

    // Sort by timestamp descending
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let total_valid_count = entries.len();
    entries.truncate(limit);
    Ok(EvidenceList {
        entries,
        invalid,
        total_valid_count,
    })
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

pub fn verify(_repo: &Path, evidence: &Evidence) -> Result<VerificationResult, WitnessError> {
    let Some(computed) = compute_bundle_hash(evidence) else {
        return Ok(VerificationResult {
            verified: false,
            reason: VerificationReason::UnsupportedHashVersion,
        });
    };

    if computed == evidence.bundle_hash {
        Ok(VerificationResult {
            verified: true,
            reason: VerificationReason::Valid,
        })
    } else {
        Ok(VerificationResult {
            verified: false,
            reason: VerificationReason::HashMismatch,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceEntry {
    pub id: String,
    pub timestamp: String,
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EvidenceList {
    pub entries: Vec<EvidenceEntry>,
    pub invalid: Vec<InvalidEvidenceEntry>,
    #[serde(default)]
    pub total_valid_count: usize,
}

impl EvidenceList {
    pub fn invalid_count(&self) -> usize {
        self.invalid.len()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvalidEvidenceEntry {
    pub path: String,
    pub reason: String,
}

#[derive(Debug)]
pub struct VerificationResult {
    pub verified: bool,
    pub reason: VerificationReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationReason {
    Valid,
    HashMismatch,
    UnsupportedHashVersion,
}

impl VerificationReason {
    pub fn as_str(self) -> &'static str {
        match self {
            VerificationReason::Valid => "valid",
            VerificationReason::HashMismatch => "hash_mismatch",
            VerificationReason::UnsupportedHashVersion => "unsupported_hash_version",
        }
    }

    pub fn human_message(self) -> &'static str {
        match self {
            VerificationReason::Valid => "bundle hash matches",
            VerificationReason::HashMismatch => "bundle hash mismatch",
            VerificationReason::UnsupportedHashVersion => "unsupported bundle hash version",
        }
    }
}

fn display_evidence_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .display()
        .to_string()
}
