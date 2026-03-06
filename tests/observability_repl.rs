use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod common;

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("json file should be readable"))
        .expect("json should parse")
}

fn phase_status<'a>(summary: &'a Value, phase_name: &str) -> Option<&'a str> {
    summary["phases"].as_array().and_then(|phases| {
        phases.iter().find_map(|phase| {
            (phase["name"].as_str() == Some(phase_name))
                .then(|| phase["status"].as_str())
                .flatten()
        })
    })
}

#[test]
fn repl_with_observability_writes_default_bundle_under_tonic_observability() {
    let fixture_root = common::unique_fixture_root("observability-repl");

    let mut child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("repl command should start");

    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b":quit\n")
        .expect("should send quit command");

    let output = child
        .wait_with_output()
        .expect("repl command should finish");

    assert!(
        output.status.success(),
        "expected repl command to succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("Tonic v"),
        "expected welcome banner in stdout"
    );

    let obs_dir = fixture_root.join(".tonic/observability");
    let latest_path = obs_dir.join("latest.json");
    assert!(
        latest_path.exists(),
        "expected latest.json at {}",
        latest_path.display()
    );

    let latest = read_json(&latest_path);
    let summary_path = PathBuf::from(
        latest["summary_path"]
            .as_str()
            .expect("latest summary path should be a string"),
    );
    assert!(summary_path.exists(), "summary file should exist");

    let summary = read_json(&summary_path);
    assert_eq!(summary["tool"]["command"], "repl");
    assert_eq!(summary["status"], "ok");
    assert_eq!(phase_status(&summary, "repl.session"), Some("ok"));
}
