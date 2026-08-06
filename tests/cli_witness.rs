//! Integration tests for witness CLI.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn witness(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_witness"));
    cmd.arg("--repo").arg(dir);
    cmd
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn json_output(output: Output, label: &str) -> serde_json::Value {
    assert_success(&output, label);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "{label} returned invalid JSON: {err}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn json_output_allow_status(
    output: Output,
    expected_status: i32,
    label: &str,
) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(expected_status),
        "{label} exited unexpectedly\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "{label} returned invalid JSON: {err}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn legacy_bundle_hash(
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
    format!("{:x}", hasher.finalize())
}

fn init_repo(dir: &Path) {
    Command::new("git")
        .arg("init")
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "witness@test.local"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Witness Test"])
        .current_dir(dir)
        .output()
        .unwrap();
    fs::write(dir.join("README.md"), "# test\n").unwrap();
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn evidence_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(".agent-witness/evidence")
        .join(format!("{id}.json"))
}

fn mutate_evidence(dir: &Path, id: &str, mut mutate: impl FnMut(&mut serde_json::Value)) {
    let path = evidence_path(dir, id);
    let content = fs::read_to_string(&path).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&content).unwrap();
    mutate(&mut value);
    fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

// --- run ---

#[test]
fn run_captures_successful_command() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "run", "--", "echo", "hello"])
            .output()
            .unwrap(),
        "witness run echo",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["exit_code"], 0);
    assert!(json["passed"].as_bool().unwrap());
    assert!(json["evidence_id"].as_str().unwrap().len() == 12);
    assert!(json["duration_ms"].as_u64().is_some());
}

#[test]
fn run_captures_failing_command() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "run", "--", "false"])
            .output()
            .unwrap(),
        "witness run false",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["passed"], false);
    assert_ne!(json["exit_code"], 0);
}

#[test]
fn run_with_tag() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let json = json_output(
        witness(dir)
            .args([
                "--format",
                "json",
                "run",
                "--tag",
                "deploy",
                "--",
                "echo",
                "deploying",
            ])
            .output()
            .unwrap(),
        "witness run with tag",
    );

    assert_eq!(json["ok"], true);

    // Verify tag appears in the stored evidence
    let id = json["evidence_id"].as_str().unwrap();
    let show = json_output(
        witness(dir)
            .args(["--format", "json", "show", id])
            .output()
            .unwrap(),
        "witness show tagged",
    );
    assert_eq!(show["evidence"]["tag"], "deploy");
}

// --- list ---

#[test]
fn list_empty_is_ok() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "list"])
            .output()
            .unwrap(),
        "witness list empty",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["evidence"].as_array().unwrap().len(), 0);
    assert_eq!(json["invalid_count"], 0);
    assert_eq!(json["invalid"].as_array().unwrap().len(), 0);
}

#[test]
fn list_shows_recorded_evidence() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    // Record two commands
    witness(dir)
        .args(["run", "--", "echo", "first"])
        .output()
        .unwrap();
    witness(dir)
        .args(["run", "--", "echo", "second"])
        .output()
        .unwrap();

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "list"])
            .output()
            .unwrap(),
        "witness list",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["evidence"].as_array().unwrap().len(), 2);
    assert_eq!(json["invalid_count"], 0);
}

#[test]
fn list_surfaces_invalid_evidence_files() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let evidence_dir = dir.join(".agent-witness/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(evidence_dir.join("bad.json"), "{not json}").unwrap();

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "list"])
            .output()
            .unwrap(),
        "witness list corrupt json",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["evidence"].as_array().unwrap().len(), 0);
    assert_eq!(json["invalid_count"], 1);
    assert!(json["invalid"][0]["path"]
        .as_str()
        .unwrap()
        .ends_with("bad.json"));
    assert!(json["invalid"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("json error"));

    let output = witness(dir).arg("list").output().unwrap();
    assert_success(&output, "witness list corrupt json text");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invalid evidence bundle"));
    assert!(stdout.contains("bad.json"));
}

// --- doctor ---

#[test]
fn doctor_empty_store_is_ready_without_creating_evidence_dir() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap(),
        "witness doctor empty",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["schema_version"], "witness.doctor.v1");
    assert_eq!(json["status"], "ready");
    assert_eq!(json["action_level"], "none");
    assert_eq!(json["evidence_count"], 0);
    assert_eq!(json["store"]["evidence_dir_exists"], false);
    assert_eq!(
        json["recommended_commands"][0]["reason_code"],
        "record_validation_evidence"
    );
    assert!(!dir.join(".agent-witness/evidence").exists());

    let strict = json_output_allow_status(
        witness(dir)
            .args(["--format", "json", "doctor", "--strict"])
            .output()
            .unwrap(),
        0,
        "witness doctor empty strict",
    );
    assert_eq!(strict["ok"], true);
    assert_eq!(strict["doctor"]["action_level"], "none");
}

#[test]
fn doctor_reports_invalid_bundles_as_review_action() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);
    let evidence_dir = dir.join(".agent-witness/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(evidence_dir.join("bad.json"), "{not json}").unwrap();

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap(),
        "witness doctor invalid",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["status"], "blocked");
    assert_eq!(json["action_level"], "review");
    assert_eq!(json["invalid_count"], 1);
    assert_eq!(json["gates"]["invalid_bundles_clear"], false);
    assert_eq!(
        json["recommended_commands"][0]["reason_code"],
        "inspect_invalid_evidence"
    );

    let strict = json_output_allow_status(
        witness(dir)
            .args(["--format", "json", "doctor", "--strict"])
            .output()
            .unwrap(),
        30,
        "witness doctor invalid strict",
    );
    assert_eq!(strict["ok"], true);
    assert_eq!(strict["doctor"]["action_level"], "review");
}

#[test]
fn doctor_blocks_when_witness_path_is_not_directory() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);
    fs::write(dir.join(".agent-witness"), "not a directory").unwrap();

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap(),
        "witness doctor blocked path",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["status"], "blocked");
    assert_eq!(json["action_level"], "stop");
    assert_eq!(json["gates"]["witness_path_usable"], false);
    assert_eq!(
        json["recommended_commands"][0]["reason_code"],
        "evidence_path_blocked"
    );
}

// --- show ---

#[test]
fn show_returns_full_evidence() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let run_json = json_output(
        witness(dir)
            .args(["--format", "json", "run", "--", "echo", "captured"])
            .output()
            .unwrap(),
        "witness run",
    );
    let id = run_json["evidence_id"].as_str().unwrap();

    let show = json_output(
        witness(dir)
            .args(["--format", "json", "show", id])
            .output()
            .unwrap(),
        "witness show",
    );

    let ev = &show["evidence"];
    assert_eq!(ev["schema_version"], 2);
    assert_eq!(ev["command"], "echo captured");
    assert_eq!(ev["command_argv"], serde_json::json!(["echo", "captured"]));
    assert_eq!(ev["bundle_hash_version"], "witness-v2");
    assert_eq!(ev["exit_code"], 0);
    assert!(ev["stdout"].as_str().unwrap().contains("captured"));
    assert!(!ev["bundle_hash"].as_str().unwrap().is_empty());
    assert!(ev["environment"]["os"].as_str().is_some());
    assert!(ev["git_context"]["branch"].as_str().is_some());
}

#[test]
fn show_nonexistent_fails() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let output = witness(dir)
        .args(["show", "nonexistent123"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code().unwrap(), 3);
}

#[test]
fn show_text_handles_short_bundle_hash() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let run_json = json_output(
        witness(dir)
            .args(["--format", "json", "run", "--", "echo", "short-hash"])
            .output()
            .unwrap(),
        "witness run",
    );
    let id = run_json["evidence_id"].as_str().unwrap();

    mutate_evidence(dir, id, |value| {
        value["bundle_hash"] = serde_json::json!("abc");
    });

    let output = witness(dir).args(["show", id]).output().unwrap();
    assert_success(&output, "witness show short hash text");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Bundle hash: abc"));
}

// --- verify ---

#[test]
fn verify_valid_bundle_passes() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let run_json = json_output(
        witness(dir)
            .args(["--format", "json", "run", "--", "echo", "integrity"])
            .output()
            .unwrap(),
        "witness run",
    );
    let id = run_json["evidence_id"].as_str().unwrap();

    let verify = json_output(
        witness(dir)
            .args(["--format", "json", "verify", id])
            .output()
            .unwrap(),
        "witness verify",
    );

    assert_eq!(verify["ok"], true);
    assert_eq!(verify["verified"], true);
    assert_eq!(verify["reason"], "valid");
}

#[test]
fn verify_tampered_bundle_fails() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let run_json = json_output(
        witness(dir)
            .args(["--format", "json", "run", "--", "echo", "tamper-test"])
            .output()
            .unwrap(),
        "witness run",
    );
    let id = run_json["evidence_id"].as_str().unwrap();

    // Tamper with the stored evidence
    let evidence_path = evidence_path(dir, id);
    let content = fs::read_to_string(&evidence_path).unwrap();
    let tampered = content.replace("tamper-test", "TAMPERED");
    fs::write(&evidence_path, tampered).unwrap();

    let verify = json_output(
        witness(dir)
            .args(["--format", "json", "verify", id])
            .output()
            .unwrap(),
        "witness verify tampered",
    );

    assert_eq!(verify["ok"], true);
    assert_eq!(verify["verified"], false);
    assert_eq!(verify["reason"], "hash_mismatch");
}

#[test]
fn verify_tampered_context_fails() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let run_json = json_output(
        witness(dir)
            .args([
                "--format", "json", "run", "--tag", "truth", "--", "echo", "context",
            ])
            .output()
            .unwrap(),
        "witness run",
    );
    let id = run_json["evidence_id"].as_str().unwrap();

    mutate_evidence(dir, id, |value| {
        value["git_context"]["dirty"] =
            serde_json::json!(!value["git_context"]["dirty"].as_bool().unwrap());
    });

    let verify = json_output(
        witness(dir)
            .args(["--format", "json", "verify", id])
            .output()
            .unwrap(),
        "witness verify context tamper",
    );

    assert_eq!(verify["ok"], true);
    assert_eq!(verify["verified"], false);
    assert_eq!(verify["reason"], "hash_mismatch");
}

#[test]
fn verify_legacy_bundle_without_hash_version_still_passes() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let evidence_dir = dir.join(".agent-witness/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();

    let id = "legacy123456";
    let command = "echo legacy";
    let timestamp = "2026-06-22T04:40:00Z";
    let exit_code = 0;
    let stdout = "legacy\n";
    let stderr = "";
    let hash = legacy_bundle_hash(command, timestamp, exit_code, stdout, stderr);
    let legacy = serde_json::json!({
        "id": id,
        "timestamp": timestamp,
        "command": command,
        "tag": "legacy",
        "cwd": dir.display().to_string(),
        "exit_code": exit_code,
        "duration_ms": 1,
        "stdout": stdout,
        "stderr": stderr,
        "environment": {
            "os": "linux",
            "user": "tester",
            "rust_version": null,
            "node_version": null
        },
        "git_context": null,
        "bundle_hash": hash
    });
    fs::write(
        evidence_dir.join(format!("{id}.json")),
        serde_json::to_string_pretty(&legacy).unwrap(),
    )
    .unwrap();

    let verify = json_output(
        witness(dir)
            .args(["--format", "json", "verify", id])
            .output()
            .unwrap(),
        "witness verify legacy",
    );

    assert_eq!(verify["ok"], true);
    assert_eq!(verify["verified"], true);
    assert_eq!(verify["reason"], "valid");
}

#[test]
fn verify_unknown_hash_version_fails_closed() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let run_json = json_output(
        witness(dir)
            .args(["--format", "json", "run", "--", "echo", "unknown-version"])
            .output()
            .unwrap(),
        "witness run",
    );
    let id = run_json["evidence_id"].as_str().unwrap();

    mutate_evidence(dir, id, |value| {
        value["bundle_hash_version"] = serde_json::json!("witness-v999");
    });

    let verify = json_output(
        witness(dir)
            .args(["--format", "json", "verify", id])
            .output()
            .unwrap(),
        "witness verify unknown version",
    );

    assert_eq!(verify["ok"], true);
    assert_eq!(verify["verified"], false);
    assert_eq!(verify["reason"], "unsupported_hash_version");
}

// --- text output ---

#[test]
fn text_output_shows_checkmark() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    let output = witness(dir)
        .args(["run", "--", "echo", "text-mode"])
        .output()
        .unwrap();

    assert_success(&output, "witness run text");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✓"));
    assert!(stdout.contains("evidence saved"));
}
