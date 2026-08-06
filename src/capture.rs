use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use crate::WitnessError;

pub const CURRENT_SCHEMA_VERSION: u32 = 2;
pub const CURRENT_BUNDLE_HASH_VERSION: &str = "witness-v2";
pub const LEGACY_BUNDLE_HASH_VERSION: &str = "legacy-v1";

#[derive(Debug, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(default = "legacy_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub timestamp: String,
    pub command: String,
    #[serde(default)]
    pub command_argv: Vec<String>,
    pub tag: Option<String>,
    pub cwd: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
    pub environment: Environment,
    pub git_context: Option<GitContext>,
    #[serde(default = "legacy_bundle_hash_version")]
    pub bundle_hash_version: String,
    pub bundle_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub os: String,
    pub user: String,
    pub rust_version: Option<String>,
    pub node_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitContext {
    pub branch: String,
    pub head_sha: String,
    pub dirty: bool,
}

pub fn run_and_capture(
    repo: &Path,
    command_parts: &[String],
    tag: Option<&str>,
) -> Result<Evidence, WitnessError> {
    if command_parts.is_empty() {
        return Err(WitnessError::Validation("No command provided".into()));
    }

    let command_argv = command_parts.to_vec();
    let full_command = command_parts.join(" ");
    let program = &command_parts[0];
    let args = &command_parts[1..];

    let start = Instant::now();
    let output = Command::new(program)
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(WitnessError::Io)?;
    let duration = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let environment = capture_environment();
    let git_context = capture_git_context(repo);

    let timestamp = Utc::now().to_rfc3339();
    let id = generate_id(&timestamp);

    let mut evidence = Evidence {
        schema_version: CURRENT_SCHEMA_VERSION,
        id,
        timestamp,
        command: full_command,
        command_argv,
        tag: tag.map(|t| t.to_string()),
        cwd: repo.display().to_string(),
        exit_code,
        duration_ms: duration.as_millis(),
        stdout,
        stderr,
        environment,
        git_context,
        bundle_hash_version: CURRENT_BUNDLE_HASH_VERSION.to_string(),
        bundle_hash: String::new(),
    };
    evidence.bundle_hash =
        compute_bundle_hash(&evidence).expect("current evidence hash version is supported");

    Ok(evidence)
}

fn capture_environment() -> Environment {
    Environment {
        os: std::env::consts::OS.to_string(),
        user: whoami_username(),
        rust_version: get_version("rustc", &["--version"]),
        node_version: get_version("node", &["--version"]),
    }
}

fn whoami_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".into())
}

fn get_version(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn capture_git_context(repo: &Path) -> Option<GitContext> {
    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;

    let head_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    Some(GitContext {
        branch,
        head_sha,
        dirty,
    })
}

fn generate_id(timestamp: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(timestamp.as_bytes());
    hasher.update(std::process::id().to_string().as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)[..12].to_string()
}

pub fn compute_bundle_hash(evidence: &Evidence) -> Option<String> {
    match evidence.bundle_hash_version.as_str() {
        CURRENT_BUNDLE_HASH_VERSION => Some(compute_current_bundle_hash(evidence)),
        LEGACY_BUNDLE_HASH_VERSION => Some(compute_legacy_bundle_hash(
            &evidence.command,
            &evidence.timestamp,
            evidence.exit_code,
            &evidence.stdout,
            &evidence.stderr,
        )),
        _ => None,
    }
}

fn compute_current_bundle_hash(evidence: &Evidence) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hash_field(
        &mut hasher,
        "schema_version",
        &evidence.schema_version.to_string(),
    );
    hash_field(&mut hasher, "id", &evidence.id);
    hash_field(&mut hasher, "timestamp", &evidence.timestamp);
    hash_field(&mut hasher, "command", &evidence.command);
    hash_field(
        &mut hasher,
        "command_argc",
        &evidence.command_argv.len().to_string(),
    );
    for (index, arg) in evidence.command_argv.iter().enumerate() {
        hash_field(&mut hasher, &format!("command_argv.{index}"), arg);
    }
    hash_optional(&mut hasher, "tag", evidence.tag.as_deref());
    hash_field(&mut hasher, "cwd", &evidence.cwd);
    hash_field(&mut hasher, "exit_code", &evidence.exit_code.to_string());
    hash_field(
        &mut hasher,
        "duration_ms",
        &evidence.duration_ms.to_string(),
    );
    hash_field(&mut hasher, "stdout", &evidence.stdout);
    hash_field(&mut hasher, "stderr", &evidence.stderr);
    hash_field(&mut hasher, "environment.os", &evidence.environment.os);
    hash_field(&mut hasher, "environment.user", &evidence.environment.user);
    hash_optional(
        &mut hasher,
        "environment.rust_version",
        evidence.environment.rust_version.as_deref(),
    );
    hash_optional(
        &mut hasher,
        "environment.node_version",
        evidence.environment.node_version.as_deref(),
    );
    match evidence.git_context.as_ref() {
        Some(git) => {
            hash_field(&mut hasher, "git_context.present", "true");
            hash_field(&mut hasher, "git_context.branch", &git.branch);
            hash_field(&mut hasher, "git_context.head_sha", &git.head_sha);
            hash_field(&mut hasher, "git_context.dirty", &git.dirty.to_string());
        }
        None => {
            hash_field(&mut hasher, "git_context.present", "false");
        }
    }
    hash_field(
        &mut hasher,
        "bundle_hash_version",
        &evidence.bundle_hash_version,
    );
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

fn compute_legacy_bundle_hash(
    command: &str,
    timestamp: &str,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(command.as_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(exit_code.to_string().as_bytes());
    hasher.update(stdout.as_bytes());
    hasher.update(stderr.as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

fn hash_optional(hasher: &mut sha2::Sha256, name: &str, value: Option<&str>) {
    match value {
        Some(value) => {
            hash_field(hasher, &format!("{name}.present"), "true");
            hash_field(hasher, name, value);
        }
        None => {
            hash_field(hasher, &format!("{name}.present"), "false");
        }
    }
}

fn hash_field(hasher: &mut sha2::Sha256, name: &str, value: &str) {
    hasher.update(name.as_bytes());
    hasher.update([0]);
    hasher.update(value.len().to_string().as_bytes());
    hasher.update([0]);
    hasher.update(value.as_bytes());
    hasher.update([0xff]);
}

fn legacy_schema_version() -> u32 {
    1
}

fn legacy_bundle_hash_version() -> String {
    LEGACY_BUNDLE_HASH_VERSION.to_string()
}
