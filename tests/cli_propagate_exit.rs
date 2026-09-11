//! `witness run --propagate-exit` exits with the wrapped command's code after
//! recording evidence; without the flag the documented MVP behaviour (exit 0)
//! is unchanged.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn witness(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_witness"));
    cmd.arg("--repo").arg(dir);
    cmd
}

#[test]
fn propagate_exit_forwards_failing_command_code() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let output = witness(dir)
        .args([
            "--format",
            "json",
            "run",
            "--propagate-exit",
            "--",
            "sh",
            "-c",
            "exit 7",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));

    // Evidence is still recorded and the normal JSON summary still printed.
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["passed"], false);
    assert_eq!(json["exit_code"], 7);
    let id = json["evidence_id"].as_str().unwrap();
    assert!(dir.join(format!(".agent-witness/evidence/{id}.json")).exists());
}

#[test]
fn propagate_exit_with_false_exits_one() {
    let tmp = TempDir::new().unwrap();
    let output = witness(tmp.path())
        .args(["run", "--propagate-exit", "--", "false"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn propagate_exit_passing_command_exits_zero() {
    let tmp = TempDir::new().unwrap();
    let output = witness(tmp.path())
        .args(["run", "--propagate-exit", "--", "true"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn without_flag_failing_command_still_exits_zero() {
    let tmp = TempDir::new().unwrap();
    let output = witness(tmp.path())
        .args(["run", "--", "false"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}
