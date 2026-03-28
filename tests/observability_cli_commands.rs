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
fn check_with_observability_preserves_stdout_contract() {
    let fixture_root = common::unique_fixture_root("observability-check-success");
    let src_dir = fixture_root.join("src");
    let obs_dir = fixture_root.join("observability");
    fs::create_dir_all(&src_dir).expect("fixture should create src dir");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture should write manifest");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture should write source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .args(["check", "."])
        .output()
        .expect("check command should execute");

    assert!(
        output.status.success(),
        "expected check command to succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "check: ok\n");
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");

    let summary = latest_summary(&obs_dir);
    assert_eq!(summary["tool"]["command"], "check");
    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["target_path"], ".");
    assert_eq!(phase_status(&summary, "check.load_source"), Some("ok"));
    assert_eq!(phase_status(&summary, "frontend.infer_types"), Some("ok"));
}

#[test]
fn check_failure_with_observability_records_normalized_error() {
    let fixture_root = common::unique_fixture_root("observability-check-failure");
    let obs_dir = fixture_root.join("observability");
    fs::write(
        fixture_root.join("bad.tn"),
        "defmodule Demo do\n  def run() do\n    if 1 do\n      true\n    end\n  end\nend\n",
    )
    .expect("fixture should write source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .args(["check", "bad.tn"])
        .output()
        .expect("check command should execute");

    assert!(!output.status.success(), "expected check command to fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("type mismatch"));

    let summary = latest_summary(&obs_dir);
    assert_eq!(summary["status"], "error");
    assert_eq!(summary["error"]["kind"], "typing_error");
    assert_eq!(summary["error"]["phase"], "frontend.infer_types");
    assert_eq!(summary["error"]["source"]["path"], "bad.tn");
    assert_eq!(summary["error"]["source"]["line"], 3);
}

#[test]
fn test_json_with_observability_preserves_machine_output() {
    let fixture_root = common::unique_fixture_root("observability-test-json");
    let tests_dir = fixture_root.join("tests");
    let obs_dir = fixture_root.join("observability");
    fs::create_dir_all(&tests_dir).expect("fixture should create tests dir");
    fs::write(
        tests_dir.join("test_math.tn"),
        "defmodule DemoTest do\n  def test_add() do\n    1 + 1\n  end\nend\n",
    )
    .expect("fixture should write test source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .args(["test", "tests", "--format", "json"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "expected test command to succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout).expect("stdout should stay json");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["passed"], 1);
    assert_eq!(report["failed"], 0);

    let summary = latest_summary(&obs_dir);
    assert_eq!(summary["tool"]["command"], "test");
    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["command_metadata"]["format"], "json");
    assert_eq!(phase_status(&summary, "test.run_suite"), Some("ok"));
}

#[test]
fn fmt_check_with_observability_preserves_failure_contract() {
    let fixture_root = common::unique_fixture_root("observability-fmt-check");
    let examples_dir = fixture_root.join("examples");
    let obs_dir = fixture_root.join("observability");
    fs::create_dir_all(&examples_dir).expect("fixture should create examples dir");
    fs::write(
        examples_dir.join("sample.tn"),
        "defmodule Demo do\ndef run() do\n1\nend\nend\n",
    )
    .expect("fixture should write source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .args(["fmt", "examples/sample.tn", "--check"])
        .output()
        .expect("fmt command should execute");

    assert!(
        !output.status.success(),
        "expected fmt --check command to fail when formatting is required"
    );
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: formatting required for 1 file (run `tonic fmt <path>` to apply fixes)\n"
    );

    let summary = latest_summary(&obs_dir);
    assert_eq!(summary["tool"]["command"], "fmt");
    assert_eq!(summary["status"], "error");
    assert_eq!(summary["command_metadata"]["mode"], "check");
    assert_eq!(summary["command_metadata"]["changed_files"], 1);
    assert_eq!(phase_status(&summary, "fmt.format_path"), Some("ok"));
}

#[test]
fn verify_and_deps_with_observability_preserve_existing_contracts() {
    let verify_root = common::unique_fixture_root("observability-verify-fail");
    let verify_obs_dir = verify_root.join("observability");
    let acceptance_dir = verify_root.join("acceptance/features");
    fs::create_dir_all(&acceptance_dir).expect("fixture should create acceptance dirs");
    fs::write(
        verify_root.join("acceptance/step-01.yaml"),
        "slice_id: step-01\nfeature_files:\n  - acceptance/features/step-01.feature\nbenchmark_metrics:\n  cold_start_p50_ms: 500\n  warm_start_p50_ms: 200\n  idle_rss_mb: 40\n",
    )
    .expect("fixture should write acceptance yaml");
    fs::write(
        verify_root.join("acceptance/features/step-01.feature"),
        "Feature: Verify slice metadata\n\n  @auto\n  Scenario: auto-smoke\n    Given tonic verify can load acceptance metadata\n",
    )
    .expect("fixture should write feature file");

    let verify_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&verify_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &verify_obs_dir)
        .args(["verify", "run", "step-01"])
        .output()
        .expect("verify command should execute");

    assert!(
        !verify_output.status.success(),
        "expected verify run to fail on benchmark thresholds"
    );
    let verify_report: Value =
        serde_json::from_slice(&verify_output.stdout).expect("verify stdout should stay json");
    assert_eq!(verify_report["status"], "fail");
    assert_eq!(verify_report["slice_id"], "step-01");

    let verify_summary = latest_summary(&verify_obs_dir);
    assert_eq!(verify_summary["tool"]["command"], "verify");
    assert_eq!(verify_summary["status"], "error");
    assert_eq!(verify_summary["target_path"], "step-01");
    assert_eq!(verify_summary["command_metadata"]["mode"], "auto");
    assert_eq!(verify_summary["error"]["kind"], "script_error");
    assert_eq!(
        phase_status(&verify_summary, "verify.load_acceptance"),
        Some("ok")
    );
    assert_eq!(
        phase_status(&verify_summary, "verify.evaluate_gates"),
        Some("error")
    );

    let deps_root = common::unique_fixture_root("observability-deps-lock");
    let deps_obs_dir = deps_root.join("observability");
    fs::create_dir_all(deps_root.join("deps/path_a")).expect("fixture should create path dep");
    fs::create_dir_all(deps_root.join("src")).expect("fixture should create src dir");
    fs::write(
        deps_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\npath_a = { path = \"deps/path_a\" }\n",
    )
    .expect("fixture should write manifest");
    fs::write(
        deps_root.join("src/main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture should write source");

    let deps_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&deps_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &deps_obs_dir)
        .args(["deps", "lock"])
        .output()
        .expect("deps command should execute");

    assert!(
        deps_output.status.success(),
        "expected deps lock to succeed, stderr: {}",
        String::from_utf8_lossy(&deps_output.stderr)
    );
    assert!(String::from_utf8_lossy(&deps_output.stdout).contains("Lockfile generated: tonic.lock"));

    let deps_summary = latest_summary(&deps_obs_dir);
    assert_eq!(deps_summary["tool"]["command"], "deps");
    assert_eq!(deps_summary["status"], "ok");
    assert_eq!(deps_summary["command_metadata"]["subcommand"], "lock");
    assert_eq!(
        phase_status(&deps_summary, "deps.load_manifest"),
        Some("ok")
    );
    assert_eq!(phase_status(&deps_summary, "deps.lock"), Some("ok"));
}
