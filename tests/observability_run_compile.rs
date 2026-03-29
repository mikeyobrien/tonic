use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

mod common;

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("json file should be readable"))
        .expect("json should parse")
}

fn latest_summary(obs_dir: &Path) -> Value {
    let latest = read_json(&obs_dir.join("latest.json"));
    let summary_path = PathBuf::from(
        latest["summary_path"]
            .as_str()
            .expect("latest summary path should be a string"),
    );
    read_json(&summary_path)
}

fn latest_artifacts(obs_dir: &Path) -> Value {
    let latest = read_json(&obs_dir.join("latest.json"));
    let summary_path = PathBuf::from(
        latest["summary_path"]
            .as_str()
            .expect("latest summary path should be a string"),
    );
    let bundle_dir = summary_path
        .parent()
        .expect("summary path should live in a bundle directory");
    read_json(&bundle_dir.join("artifacts.json"))
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

fn artifact_kinds(artifacts: &Value) -> Vec<&str> {
    artifacts["items"]
        .as_array()
        .expect("artifact manifest items should be an array")
        .iter()
        .map(|item| {
            item["kind"]
                .as_str()
                .expect("artifact kind should be a string")
        })
        .collect()
}

#[test]
fn run_with_observability_emits_bundle_with_phase_timings() {
    let fixture_root = common::unique_fixture_root("observability-run-success");
    let source_path = fixture_root.join("demo.tn");
    let obs_dir = fixture_root.join("observability");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    40 + 2\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .args(["run", "demo.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected observed run to succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("stdout should be utf8")
            .trim_end(),
        "42"
    );

    let summary = latest_summary(&obs_dir);
    assert_eq!(summary["schema_name"], "tonic.observability.run");
    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["tool"]["command"], "run");
    assert_eq!(summary["target_path"], "demo.tn");
    assert_eq!(phase_status(&summary, "run.load_source"), Some("ok"));
    assert_eq!(phase_status(&summary, "frontend.parse_ast"), Some("ok"));
    assert_eq!(
        phase_status(&summary, "run.evaluate_entrypoint"),
        Some("ok")
    );
}

#[test]
fn compile_with_observability_records_artifacts_and_keeps_legacy_profile_output() {
    let fixture_root = common::unique_fixture_root("observability-compile-success");
    let source_path = fixture_root.join("native.tn");
    let obs_dir = fixture_root.join("observability");
    let profile_path = fixture_root.join("profile.jsonl");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    7 * 6\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .env("TONIC_PROFILE_OUT", &profile_path)
        .args(["compile", "native.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected observed compile to succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary = latest_summary(&obs_dir);
    let artifacts = latest_artifacts(&obs_dir);
    let kinds = artifact_kinds(&artifacts);

    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["tool"]["command"], "compile");
    assert_eq!(summary["legacy_signals"]["profile_enabled"], true);
    assert_eq!(phase_status(&summary, "compile.load_source"), Some("ok"));
    assert_eq!(
        phase_status(&summary, "backend.link_executable"),
        Some("ok")
    );
    assert!(kinds.contains(&"native-executable"));
    assert!(kinds.contains(&"c-source"));
    assert!(kinds.contains(&"ir-sidecar"));
    assert!(kinds.contains(&"native-manifest"));

    let profile_line = fs::read_to_string(&profile_path)
        .expect("profile output should exist")
        .lines()
        .next()
        .expect("profile output should contain one json line")
        .to_string();
    let profile: Value = serde_json::from_str(&profile_line).expect("profile line should be json");
    assert_eq!(profile["command"], "compile");
}

#[test]
fn compile_failure_with_observability_records_normalized_error_and_source() {
    let fixture_root = common::unique_fixture_root("observability-compile-failure");
    let source_path = fixture_root.join("bad.tn");
    let obs_dir = fixture_root.join("observability");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    if 1 do\n      true\n    end\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .args(["compile", "bad.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        !output.status.success(),
        "expected observed compile failure, stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let summary = latest_summary(&obs_dir);
    assert_eq!(summary["status"], "error");
    assert_eq!(summary["error"]["kind"], "typing_error");
    assert_eq!(summary["error"]["diagnostic_code"], "E2001");
    assert_eq!(summary["error"]["phase"], "frontend.infer_types");
    assert!(summary["error"]["message"]
        .as_str()
        .expect("error message should be a string")
        .contains("type mismatch"));
    assert_eq!(summary["error"]["source"]["path"], "bad.tn");
    assert_eq!(summary["error"]["source"]["line"], 3);
    assert_eq!(
        phase_status(&summary, "frontend.infer_types"),
        Some("error")
    );
}
