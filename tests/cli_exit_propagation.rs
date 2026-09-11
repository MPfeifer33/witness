//! `witness run` exits with the wrapped command's code after recording evidence
//! (the default since 2026-09-11); `--exit-zero` restores the legacy always-0 exit.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn witness(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_witness"));
    cmd.arg("--repo").arg(dir);
    cmd
}

#[test]
fn failing_command_code_is_forwarded_by_default() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let output = witness(dir)
        .args(["--format", "json", "run", "--", "sh", "-c", "exit 7"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));

    // Evidence is still recorded and the normal JSON summary still printed.
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["passed"], false);
    assert_eq!(json["exit_code"], 7);
    let id = json["evidence_id"].as_str().unwrap();
    assert!(dir
        .join(format!(".agent-witness/evidence/{id}.json"))
        .exists());
}

#[test]
fn false_exits_one_by_default() {
    let tmp = TempDir::new().unwrap();
    let output = witness(tmp.path())
        .args(["run", "--", "false"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn passing_command_exits_zero() {
    let tmp = TempDir::new().unwrap();
    let output = witness(tmp.path())
        .args(["run", "--", "true"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn exit_zero_flag_restores_legacy_behaviour() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let output = witness(dir)
        .args(["--format", "json", "run", "--exit-zero", "--", "false"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["passed"], false);
    assert!(dir
        .join(format!(
            ".agent-witness/evidence/{}.json",
            json["evidence_id"].as_str().unwrap()
        ))
        .exists());
}

#[test]
fn propagate_exit_flag_is_gone() {
    let tmp = TempDir::new().unwrap();
    let output = witness(tmp.path())
        .args(["run", "--propagate-exit", "--", "true"])
        .output()
        .unwrap();
    assert_ne!(output.status.code(), Some(0));
}
