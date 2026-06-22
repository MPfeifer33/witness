//! Integration tests for witness CLI.

use std::fs;
use std::path::Path;
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
            .args(["--format", "json", "run", "--tag", "deploy", "--", "echo", "deploying"])
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
}

#[test]
fn list_shows_recorded_evidence() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    init_repo(dir);

    // Record two commands
    witness(dir).args(["run", "--", "echo", "first"]).output().unwrap();
    witness(dir).args(["run", "--", "echo", "second"]).output().unwrap();

    let json = json_output(
        witness(dir)
            .args(["--format", "json", "list"])
            .output()
            .unwrap(),
        "witness list",
    );

    assert_eq!(json["ok"], true);
    assert_eq!(json["evidence"].as_array().unwrap().len(), 2);
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
    assert_eq!(ev["command"], "echo captured");
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
    let evidence_path = dir.join(".agent-witness/evidence").join(format!("{id}.json"));
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
