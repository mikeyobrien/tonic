use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
mod common;

#[test]
fn verify_auto_mode_includes_only_auto_tagged_scenarios() {
    let fixture_root = write_verify_fixture("verify-auto-mode-filter");

    let report = run_verify_report(&fixture_root, "auto");

    assert_eq!(report["mode"], "auto");
    assert_eq!(scenario_ids(&report), vec!["auto-smoke".to_string()]);
}

#[test]
fn verify_mixed_mode_excludes_human_manual_scenarios() {
    let fixture_root = write_verify_fixture("verify-mixed-mode-filter");

    let report = run_verify_report(&fixture_root, "mixed");

    assert_eq!(report["mode"], "mixed");
    assert_eq!(
        scenario_ids(&report),
        vec!["auto-smoke".to_string(), "agent-review".to_string()]
    );
}

#[test]
fn verify_manual_mode_includes_all_tagged_scenarios() {
    let fixture_root = write_verify_fixture("verify-manual-mode-filter");

    let report = run_verify_report(&fixture_root, "manual");

    assert_eq!(report["mode"], "manual");
    assert_eq!(
        scenario_ids(&report),
        vec![
            "auto-smoke".to_string(),
            "agent-review".to_string(),
            "human-ux".to_string()
        ]
    );
}

fn run_verify_report(fixture_root: &Path, mode: &str) -> Value {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .args(["verify", "run", "step-13", "--mode", mode])
        .output()
        .expect("verify command should execute");

    assert!(
        output.status.success(),
        "expected verify command to succeed, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("verify stdout should be utf8");
    serde_json::from_str(stdout.trim()).expect("verify output should be valid json")
}

fn scenario_ids(report: &Value) -> Vec<String> {
    report["scenarios"]
        .as_array()
        .expect("verify report scenarios should be an array")
        .iter()
        .map(|scenario| {
            scenario["id"]
                .as_str()
                .expect("scenario id should be a string")
                .to_string()
        })
        .collect()
}

fn write_verify_fixture(test_name: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let acceptance_dir = fixture_root.join("acceptance/features");

    fs::create_dir_all(&acceptance_dir)
        .expect("fixture setup should create acceptance/features directory");

    fs::write(
        fixture_root.join("acceptance/step-13.yaml"),
        "slice_id: step-13\nfeature_files:\n  - acceptance/features/step-13.feature\n",
    )
    .expect("fixture setup should write acceptance yaml");

    fs::write(
        fixture_root.join("acceptance/features/step-13.feature"),
        "Feature: Verify mode filtering\n\n  @auto\n  Scenario: auto-smoke\n    Given auto coverage exists\n\n  @agent-manual\n  Scenario: agent-review\n    Given agent-manual evidence exists\n\n  @human-manual\n  Scenario: human-ux\n    Given human verification evidence exists\n",
    )
    .expect("fixture setup should write feature file");

    fixture_root
}
