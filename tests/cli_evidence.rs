//! Integration tests for evidence recording, listing, showing, and verifying.

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
        String::from_utf8_lossy(&output.stderr)
    );
}

fn json_output(output: Output, label: &str) -> serde_json::Value {
    assert_success(&output, label);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "{label} returned invalid json: {err}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn evidence_roundtrip_records_lists_shows_and_verifies() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let run = json_output(
        witness(dir)
            .args([
                "--format",
                "json",
                "run",
                "--tag",
                "smoke",
                "--",
                "sh",
                "-c",
                "printf hello",
            ])
            .output()
            .unwrap(),
        "witness run",
    );

    let id = run["evidence_id"].as_str().unwrap();
    assert_eq!(run["passed"], true);
    assert_eq!(run["exit_code"], 0);
    assert!(dir
        .join(format!(".agent-witness/evidence/{id}.json"))
        .exists());

    let list = json_output(
        witness(dir)
            .args(["--format", "json", "list"])
            .output()
            .unwrap(),
        "witness list",
    );
    assert_eq!(list["evidence"][0]["id"], id);
    assert_eq!(list["evidence"][0]["tag"], "smoke");

    let show = json_output(
        witness(dir)
            .args(["--format", "json", "show", id])
            .output()
            .unwrap(),
        "witness show",
    );
    assert_eq!(show["evidence"]["id"], id);
    assert_eq!(show["evidence"]["stdout"], "hello");
    assert_eq!(show["evidence"]["tag"], "smoke");
    assert!(!show["evidence"]["bundle_hash"].as_str().unwrap().is_empty());

    let verify = json_output(
        witness(dir)
            .args(["--format", "json", "verify", id])
            .output()
            .unwrap(),
        "witness verify",
    );
    assert_eq!(verify["verified"], true);
}

#[test]
fn failed_command_is_recorded_without_failing_witness() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let run = json_output(
        witness(dir)
            .args([
                "--format",
                "json",
                "run",
                "--tag",
                "failure",
                "--",
                "sh",
                "-c",
                "echo nope >&2; exit 7",
            ])
            .output()
            .unwrap(),
        "witness run failed command",
    );

    let id = run["evidence_id"].as_str().unwrap();
    assert_eq!(run["passed"], false);
    assert_eq!(run["exit_code"], 7);

    let show = json_output(
        witness(dir)
            .args(["--format", "json", "show", id])
            .output()
            .unwrap(),
        "witness show failure",
    );
    assert_eq!(show["evidence"]["exit_code"], 7);
    assert!(show["evidence"]["stderr"]
        .as_str()
        .unwrap()
        .contains("nope"));
}
