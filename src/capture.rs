use std::path::Path;
use std::process::Command;
use std::time::Instant;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::WitnessError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub timestamp: String,
    pub command: String,
    pub tag: Option<String>,
    pub cwd: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
    pub environment: Environment,
    pub git_context: Option<GitContext>,
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

    let full_command = command_parts.join(" ");
    let program = &command_parts[0];
    let args = &command_parts[1..];

    let start = Instant::now();
    let output = Command::new(program)
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| WitnessError::Io(e))?;
    let duration = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let environment = capture_environment();
    let git_context = capture_git_context(repo);

    let timestamp = Utc::now().to_rfc3339();
    let id = generate_id(&timestamp);

    // Compute bundle hash over key fields
    let bundle_hash = compute_bundle_hash(&full_command, &timestamp, exit_code, &stdout, &stderr);

    Ok(Evidence {
        id,
        timestamp,
        command: full_command,
        tag: tag.map(|t| t.to_string()),
        cwd: repo.display().to_string(),
        exit_code,
        duration_ms: duration.as_millis(),
        stdout,
        stderr,
        environment,
        git_context,
        bundle_hash,
    })
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

    Some(GitContext { branch, head_sha, dirty })
}

fn generate_id(timestamp: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(timestamp.as_bytes());
    hasher.update(std::process::id().to_string().as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)[..12].to_string()
}

fn compute_bundle_hash(command: &str, timestamp: &str, exit_code: i32, stdout: &str, stderr: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(command.as_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(exit_code.to_string().as_bytes());
    hasher.update(stdout.as_bytes());
    hasher.update(stderr.as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)
}
