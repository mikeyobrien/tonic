use std::fs;
use std::path::PathBuf;
use std::process::Command;
mod common;

#[test]
fn native_regression_policy_passes_green_contract() {
    let fixture_root = common::unique_fixture_root("native-regression-policy-pass");
    let report_path = fixture_root.join("summary.json");
    fs::write(&report_path, passing_report_json()).expect("fixture should write summary report");

    let output = run_policy_script(&report_path);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected pass exit code, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("verdict=pass"),
        "expected pass verdict in stdout, got: {stdout}"
    );
}

#[test]
fn native_regression_policy_marks_small_regression_as_quarantine() {
    let fixture_root = common::unique_fixture_root("native-regression-policy-quarantine");
    let report_path = fixture_root.join("summary.json");
    fs::write(&report_path, quarantine_report_json()).expect("fixture should write summary report");

    let output = run_policy_script(&report_path);

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected quarantine exit code, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("verdict=quarantine"),
        "expected quarantine verdict in stdout, got: {stdout}"
    );
}

#[test]
fn native_regression_policy_marks_large_regression_as_rollback() {
    let fixture_root = common::unique_fixture_root("native-regression-policy-rollback");
    let report_path = fixture_root.join("summary.json");
    fs::write(&report_path, rollback_report_json()).expect("fixture should write summary report");

    let output = run_policy_script(&report_path);

    assert_eq!(
        output.status.code(),
        Some(3),
        "expected rollback exit code, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("verdict=rollback"),
        "expected rollback verdict in stdout, got: {stdout}"
    );
}

fn run_policy_script(report_path: &std::path::Path) -> std::process::Output {
    Command::new("bash")
        .arg(policy_script_path())
        .arg(report_path)
        .output()
        .expect("policy script should execute")
}

fn policy_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/native-regression-policy.sh")
}

fn passing_report_json() -> String {
    r#"{
  "status": "pass",
  "performance_contract": {
    "status": "pass",
    "relative_budget_pct": 25,
    "pass_threshold": 0.82,
    "overall_score": 0.88,
    "failure_reasons": [],
    "slo": {
      "status": "pass",
      "failures": []
    },
    "workload_scores": [
      {
        "name": "run_control",
        "status": "pass",
        "p50_ratio_to_best_ref": 1.11,
        "p95_ratio_to_best_ref": 1.15,
        "rss_ratio_to_best_ref": 1.2
      }
    ]
  }
}"#
    .to_string()
}

fn quarantine_report_json() -> String {
    r#"{
  "status": "fail",
  "performance_contract": {
    "status": "fail",
    "relative_budget_pct": 25,
    "pass_threshold": 0.82,
    "overall_score": 0.8,
    "failure_reasons": [
      "workload 'run_control' exceeded relative budget 25.0% compared to references"
    ],
    "slo": {
      "status": "pass",
      "failures": []
    },
    "workload_scores": [
      {
        "name": "run_control",
        "status": "fail",
        "p50_ratio_to_best_ref": 1.32,
        "p95_ratio_to_best_ref": 1.31,
        "rss_ratio_to_best_ref": 1.18
      }
    ]
  }
}"#
    .to_string()
}

fn rollback_report_json() -> String {
    r#"{
  "status": "fail",
  "performance_contract": {
    "status": "fail",
    "relative_budget_pct": 25,
    "pass_threshold": 0.82,
    "overall_score": 0.66,
    "failure_reasons": [
      "overall score 0.660 is below pass threshold 0.820",
      "workload 'run_control' exceeded relative budget 25.0% compared to references"
    ],
    "slo": {
      "status": "fail",
      "failures": [
        "runtime_p95_ms 56 exceeded threshold 30"
      ]
    },
    "workload_scores": [
      {
        "name": "run_control",
        "status": "fail",
        "p50_ratio_to_best_ref": 1.6,
        "p95_ratio_to_best_ref": 1.7,
        "rss_ratio_to_best_ref": 1.4
      }
    ]
  }
}"#
    .to_string()
}
